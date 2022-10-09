## 本质
&emsp;`channel`在本质上是一个带有一个循环队列,两个收发队列,一个锁的数据结构.当然channel也遵循FIFO的原则,针对无bufferchannel,数据优先直达,针对有buffer的channel,数据优先从buffer里面,然后在再是收发队列里面的sudog.

### 数据结构
&emsp;channel在runtime里面的结构如下:
```go 
type hchan struct {
	qcount   uint           // channel中元素的个数
	dataqsiz uint           // channel中循环队列的长度
	buf      unsafe.Pointer // channel中缓冲区数据指针
	elemsize uint16              // channel能够收发操作元素的大小
	closed   uint32
	elemtype *_type //            // channel 收发操作元素的类型
	sendx    uint   // channel的发送操作处理到的位置
	recvx    uint   // channel的接收操作处理到的位置
	recvq    waitq  // 存储了当前channel由于缓冲区不足而阻塞的goroutine
	sendq    waitq  // 同recvq 
	lock mutex
}
// waitq等待队列,双向连表
type waitq struct {
	first *sudog
	last  *sudog
}
// sudog 是一个双向连表, sudog表示一个在等待列表里面中的goroutine,连表里面的所有的元素都是sudog
type sudog struct {
	// The following fields are protected by the hchan.lock of the
	// channel this sudog is blocking on. shrinkstack depends on
	// this for sudogs involved in channel ops.
	g *g

	next *sudog
	prev *sudog
	elem unsafe.Pointer // data element (may point to stack)

	acquiretime int64
	releasetime int64
	ticket      uint32

	// isSelect indicates g is participating in a select, so
	// g.selectDone must be CAS'd to win the wake-up race.
	isSelect bool

	parent   *sudog // semaRoot binary tree
	waitlink *sudog // g.waiting list or semaRoot
	waittail *sudog // semaRoot
	c        *hchan // channel
}
```

### 创建管道
&emsp;`make(chan int ,10)`,编译器会将make转换为`OMAKE`节点,并且在类型检查里面将`OMAKE`转换为`OMAKECHAN`类型:
```go 
func typecheck1(n *Node, top int) (res *Node) {
	if enableTrace && trace {
		defer tracePrint("typecheck1", n)(&res)
	}

	switch n.Op {
	case OMAKE:
		switch t.Etype {
		case TCHAN:     // channel 类型
			l = nil
			if i < len(args) {  // 带缓冲区的异步channel(参数个数大于1 make(type, args))
				n.Left = l
			} else {                            //  不带缓冲区的同步channel
				n.Left = nodintconst(0)
			}
			n.Op = OMAKECHAN  // 转换为OMAKECHAN 节点
		}
    }
}

// walkexpr 会在SSA 代码生成之前转换成`runtime.makechan`或者`runtime.makechan64`的函数:
func walkexpr(n *Node, init *Nodes) *Node {
	if n == nil {
		return n
	}
	switch n.Op {
	case OMAKECHAN:
		// When size fits into int, use makechan instead of
		// makechan64, which is faster and shorter on 32 bit platforms.
		size := n.Left
		fnname := "makechan64"      // 适用于缓冲区大于2^32次方的大小
		argtype := types.Types[TINT64]

		if size.Type.IsKind(TIDEAL) || maxintval[size.Type.Etype].Cmp(maxintval[TUINT]) <= 0 {
			fnname = "makechan"
			argtype = types.Types[TINT]
		}
		n = mkcall1(chanfn(fnname, 1, n.Type), n.Type, init, typename(n.Type), conv(size, argtype))
    }
    // 正常情况下一般调用的makechan函数
}
```
&emsp;`makechan`的实现:
```go 
func makechan(t *chantype, size int) *hchan {
	elem := t.elem
	// compiler checks this but be safe.
	if elem.size >= 1<<16 { // 类型大小检查
		throw("makechan: invalid channel element type")
	}
	mem, overflow := math.MulUintptr(elem.size, uintptr(size))  // 计算所需要的内存大小

	var c *hchan
	switch {
	case mem == 0:      // 不存在缓冲区情况
		// Queue or element size is zero.
		c = (*hchan)(mallocgc(hchanSize, nil, true))        // 
		// Race detector uses this location for synchronization.
		c.buf = c.raceaddr()
	case elem.ptrdata == 0:         // 缓冲区里面的元素类型不是指针类型,hchan和buffer的分配都在一个调用里面完成
		// Elements do not contain pointers.
		// Allocate hchan and buf in one call.
		c = (*hchan)(mallocgc(hchanSize+mem, nil, true))
		c.buf = add(unsafe.Pointer(c), hchanSize)  // 申请一段连续的内存空间(类似sbrk直接增加)
	default:                       //  缓冲区里面元素的类型是指针类型
		// Elements contain pointers. // 元素包含指针
		c = new(hchan)
		c.buf = mallocgc(mem, elem, true)       // 在堆上申请内存空间
	}

	c.elemsize = uint16(elem.size)
	c.elemtype = elem
	c.dataqsiz = uint(size)
	lockInit(&c.lock, lockRankHchan)
	return c
}
```

### 发送数据
&emsp;向channel发送数据的语法`ch <- i`,编译器会将它解析成`OSEND`节点,并且在`runtime.walkexpr`里面将`OSEND`节点转换成`runtime.chansend1`;j
&emsp;先说结论,发送数据逻辑如下:
- 无缓冲区channel,但是存在接收者----- 直接将元素复制给接受者sudog对应的内存位置
- 无缓冲区channel,但是没有接受者----- 将自己goroutine封装成sudog,加入到sendq队列里面,等待其他接收goroutine来唤醒
- 有缓冲区channel,缓冲区未满--------- 直接将元素添加到环形队列里面,更新环形队列索引
- 有缓冲区channel,缓冲区已满--------- 将字节goroutine封装成sudog,加入到sendq队列里面,等待其他接收goroutine来唤醒

```go 
func walkexpr(n *Node, init *Nodes) *Node {
	switch n.Op {
	case OSEND:
		n1 := n.Right
		n1 = assignconv(n1, n.Left.Type.Elem(), "chan send")
		n1 = walkexpr(n1, init)
		n1 = nod(OADDR, n1, nil)
		n = mkcall1(chanfn("chansend1", 2, n.Left.Type), nil, init, n.Left, n1)
    }
}
// chansend1 ---> chansend
func chansend1(c *hchan, elem unsafe.Pointer) {
	chansend(c, elem, true, getcallerpc())
}
//  chansend 实际发送函数(当向channel发送数据时候就会调用这个函数)
func chansend(c *hchan, ep unsafe.Pointer, block bool, callerpc uintptr) bool {
    // todo
	lock(&c.lock) // 在下面执行发送数据之前会对当前的channel执行加锁的操作

	if c.closed != 0 { //如果channel被关闭，那么它会抛出异常
		unlock(&c.lock)
		panic(plainError("send on closed channel"))
	}
}
```


#### Case1 直接发送(同步,阻塞)
&esmp;如果目标channel没有被关闭,并且已经有处于读等待的goroutine, 则`runtime.chansend`会从接收队列`recvq`中取出最先陷入等待的goroutine并直接向它发送数据
```go 
// entry point for c <- x from compiled code
//go:nosplit
func chansend1(c *hchan, elem unsafe.Pointer) {
    // chan, ep, block set to true
	chansend(c, elem, true, getcallerpc())
}
func chansend(c *hchan, ep unsafe.Pointer, block bool, callerpc uintptr) bool {
    // block is set to true
	if !block && c.closed == 0 && full(c) { //如果非阻塞 && chan没有关闭 && channel缓冲区已经满
        // 非阻塞情况下,channel已经满了直接returnfalse , 当然在这个里面block已经被设置了true
		return false
	}
    ...
    // 这个情况是针对无buffer
	if sg := c.recvq.dequeue(); sg != nil { // 如果接收对了里面存在sudog,并且sudog不为nil, 调用send方法
		// Found a waiting receiver. We pass the value we want to send
		// directly to the receiver, bypassing the channel buffer (if any).
		send(c, sg, ep, func() { unlock(&c.lock) }, 3)
		return true
	}
}
func send(c *hchan, sg *sudog, ep unsafe.Pointer, unlockf func(), skip int) {
    //  sendDirect操作会直接通过memmove将ep内存copy到sg.elem
	sendDirect(c.elemtype, sg, ep) // sendDirect(type, dest, source)
    //  将gp设置为可以调度状态,(G状态设置为)
	gp := sg.g
	goready(gp, skip+1) 
}
// goready的操作将go的状态设置为_Grunnable,并且把它方法当前m的runnext里面
func goready(gp *g, traceskip int) {
	casgstatus(gp, _Gwaiting, _Grunnable)
	runqput(_g_.m.p.ptr(), gp, next)
}
```
> 逻辑如下
```txt 
                      ┌──────┐                   ┌─────────┐
                      │ Data │                   │ Channle │
             ┌──────► └──────┘                   └─────────┘
             │                                        │
             │ 2将data发送给sudog                     │
             │                                        ▼
             │                                 ┌──────────┐
             │                                 │  RecvQ   │
             │                                 └──────────┘
      ┌───────────┐                                ▲
      │   Send    │ ───────────────────────────────┘
      └───────────┘              (1)遍历recvq连表,找到不为空的sudog

```

> 这里逻辑发现接收队列里面存在sudog，那么直接把data直接copy到sudog里面，然后将sudog加入到q.runnext队列里面等待下一次调度

#### 缓冲区(有缓冲区意味着是异步的,非阻塞)
&emsp;如果创建的channel包含了缓冲区,并且channel中的数据没有装满,会执行下面的代码:
```go
func chansend(c *hchan, ep unsafe.Pointer, block bool, callerpc uintptr) bool {
    // 当元素 < 队列长度(缓冲区还没有被装满)
	if c.qcount < c.dataqsiz {
		// Space is available in the channel buffer. Enqueue the element to send.
	    // return add(c.buf, uintptr(i)*uintptr(c.elemsize))
		qp := chanbuf(c, c.sendx) //  计算下一个可以存储数据的位置
        // 通过typedmemmove 将 ep copy到qp这个位置
		typedmemmove(c.elemtype, qp, ep)
        // 自增send 和qcount 字段
		c.sendx++
		c.qcount++
		unlock(&c.lock)
		return true
	}
}
```
> 当往buffer的channel里面发送数据，如果buffer没有满,直接将元素添加到buffer里面就可以,然后直接返回true

#### 阻塞发送
&emsp;当channel没有接收者能够处理数据时,向channel发送数据会被下游阻塞,(当然可以通过select关键字可以向channel非阻塞的发送消息),向Chanel阻塞的发送数据会执行下面的逻辑:
```go
func chansend(c *hchan, ep unsafe.Pointer, block bool, callerpc uintptr) bool {
	gp := getg()                        // 获取发送数据的goroutine
	mysg := acquireSudog()              // runtime.acquireSudog 会申请一个新的sudog 结构体(会调用malloc,malloc会call gc,gc cause stw)
    // 给创建的sudog 设置一些属性
	mysg.releasetime = 0
	if t0 != 0 {
		mysg.releasetime = -1
	}
	// No stack splits between assigning elem and enqueuing mysg
	// on gp.waiting where copystack can find it.
	mysg.elem = ep                  // 需要发送的数据
	mysg.waitlink = nil         
	mysg.g = gp                     // 发送数据的g
	mysg.isSelect = false           // 是不是在select 里面
	mysg.c = c                      // 当前channel地址
	gp.waiting = mysg
	gp.param = nil
    // 当前的goroutine 加入到channel sendq队列里面
	c.sendq.enqueue(mysg)
	// Signal to anyone trying to shrink our stack that we're about
	// to park on a channel. The window between when this G's status
	// changes and when we set gp.activeStackChans is not safe for
	// stack shrinking.
	atomic.Store8(&gp.parkingOnChan, 1)
	gopark(chanparkcommit, unsafe.Pointer(&c.lock), waitReasonChanSend, traceEvGoBlockSend, 2)
	// Ensure the value being sent is kept alive until the
	// receiver copies it out. The sudog has a pointer to the
	// stack object, but sudogs aren't considered as roots of the
	// stack tracer.
	KeepAlive(ep)

	// someone woke us up.
	if mysg != gp.waiting {
		throw("G waiting list is corrupted")
	}
    // goroutine 被重新唤醒(表示数据已经被成功发送出去了,下面开始执行一些清理的动作)
	gp.waiting = nil
	gp.activeStackChans = false
	if gp.param == nil {
		if c.closed == 0 {
			throw("chansend: spurious wakeup")
		}
		panic(plainError("send on closed channel"))
	}
	gp.param = nil
	if mysg.releasetime > 0 {
		blockevent(mysg.releasetime-t0, 2)
	}
	mysg.c = nil
	releaseSudog(mysg)
	return true
}
```
&emsp;主要流程如下:
1.  获取当前goroutine
2. 调用`runtime.acquireSudog` 申请一个新的sudog
3. 将当前上下文封装到sudog里面
4. 将sudog加入到当前发送队列里面(c.sendq)
5. 当前goroutine陷入沉睡等待唤醒
6. 被调度器唤醒后会执行一些收尾工作


### 接收数据
&emsp;先说接收数据流程
- 存在接收队列, 无缓冲区channel         ---> 直接将sudog.elem 复制给接收端
- 存在接收队列, 有缓冲区channel         ---> 先将buffer里面数据复制给接收端,再把sudog.elem 元素添加到buffer里面(遵循先进先出原则)
- 不存在接收队列, 无缓冲区channel       ---> 将当前g封装成sudog, 等待被唤醒
- 不存在接收队列, 有缓冲区channel,buffer有数据 --> 当buffer里面元素复制给接收者,更新索引
- 不存在接收队列, 有缓冲区channel,buffer无数据 --> 将当前g封装成sudog,等待被唤醒

&emsp;go接收数据有两种方式:
1. `i <- ch`
2. `i, ok <- ch`
&emsp;这两种方式都会被编译器处理成`ORECV`节点(后面一种会在类型检查阶段被转换为`OAS2RECV`)

```text
                                    CHANNEL   RECEIVE   NODE

                     ┌─────────┐    ┌──────────┐            ┌──────────────┐
                     │  <- ch  ├───►│   ORECV  │───────────►│   chanreve1  │────────┐       
                     └─────────┘    └─────┬────┘            └──────────────┘        │   ┌───────────┐
                                          ▼                                         ├──►│  chanrecv │
                                  ┌────────────┐            ┌──────────────┐        │   └───────────┘
                                  │  OAS2RECV  │───────────►│   chanreve2  │────────┘
                                  └────────────┘            └──────────────┘
```
&emsp;最终都是调用`chanrecv`函数
```go
func chanrecv(c *hchan, ep unsafe.Pointer, block bool) (selected, received bool) {
	if c == nil {       //  从一个空的channel里面recv数据会让出处理器使用权
		if !block {
			return
		}
		gopark(nil, nil, waitReasonChanReceiveNilChan, traceEvGoStop, 2)
		throw("unreachable")
	}
	lock(&c.lock)       //锁定当前的channel

    // 如果channel已经关闭了,并且channel里面已经没有元素了
	if c.closed != 0 && c.qcount == 0 {
		unlock(&c.lock)             // 解锁channel
		if ep != nil {
			typedmemclr(c.elemtype, ep)     // 清理内存
		}
		return true, false
	}

}
```


#### 直接接收数据
&emsp;当channel的sendq队列中包含处于等待状态的goroutine的时候,该函数会直接取出队头等待的goroutine,(处理逻辑和发送没啥差别):
```go
func chanrecv(c *hchan, ep unsafe.Pointer, block bool) (selected, received bool) {
	if sg := c.sendq.dequeue(); sg != nil {
		recv(c, sg, ep, func() { unlock(&c.lock) }, 3)
		return true, true
	}
}
// 接收函数实现
func recv(c *hchan, sg *sudog, ep unsafe.Pointer, unlockf func(), skip int) {
if c.dataqsiz == 0 {    // 队列长度为0情况(不存在缓冲区情况下)
		if ep != nil {
			// copy data from sender
			recvDirect(c.elemtype, sg, ep)      // 直接将channelchannel的buffer通过memmove复制到sudo.elem里面
		}
	} else {        // 存在缓冲区的情况
		qp := chanbuf(c, c.recvx)       // 从recvx获取
		// copy data from queue to receiver
		if ep != nil {                              // 如果接收方的元素不为空,则将channel receiver里面的元素copy到ep里面
			typedmemmove(c.elemtype, ep, qp)
		}
		// copy data from sender to queue           // 将发送队列的头部数据copy到缓冲区里面
		typedmemmove(c.elemtype, qp, sg.elem)
		c.recvx++                                   // 释放一个发送方
		if c.recvx == c.dataqsiz {
			c.recvx = 0
		}
		c.sendx = c.recvx // c.sendx = (c.sendx+1) % c.dataqsiz
	}
	sg.elem = nil
	gp := sg.g
	unlockf()
	gp.param = unsafe.Pointer(sg)
	if sg.releasetime != 0 {
		sg.releasetime = cputicks()
	}
	goready(gp, skip+1)             // 将发送数据的goroutine设置可以调度
}

func ready(gp *g, traceskip int, next bool) {
	status := readgstatus(gp)
	casgstatus(gp, _Gwaiting, _Grunnable)
	runqput(_g_.m.p.ptr(), gp, next)
	wakep()
	releasem(mp)
}
```
&emsp;从代码看，无论是那种情况下，都会触发goready当当前处理器的runnext 设置为发送数据的goroutine(从接收队列里面获取sudog)

#### 缓冲区
&emsp;如果创建的channel包含了缓冲区,但是缓冲区的数据并没有被装满,则会跳转到下面的部分:
```go
func chanrecv(c *hchan, ep unsafe.Pointer, block bool) (selected, received bool) {
	if c.qcount > 0 {
		// Receive directly from queue
		qp := chanbuf(c, c.recvx)       // 计算下一个可以取数据的位置
		if ep != nil {                  // 如果接收队列不为空,则直接从receive buffer里面直接copy到ep内存位置
			typedmemmove(c.elemtype, ep, qp)
		}
		typedmemclr(c.elemtype, qp)             // 清理buffer里面已经被接收的数据(释放内存)
		c.recvx++                               // recvx ++ 
		if c.recvx == c.dataqsiz {
			c.recvx = 0
		}
		c.qcount--                              // 队列-1
		unlock(&c.lock)
		return true, true
	}
}
```

#### 阻塞接收
&emsp; 当channel
```go
func chanrecv(c *hchan, ep unsafe.Pointer, block bool) (selected, received bool) {
	// no sender available: block on this channel.
	gp := getg()
	mysg := acquireSudog()  //  acquireSudog 申请一个sudog
    // 填充sudog上下文
	mysg.releasetime = 0
	if t0 != 0 {
		mysg.releasetime = -1
	}
	// No stack splits between assigning elem and enqueuing mysg
	// on gp.waiting where copystack can find it.
	mysg.elem = ep
	mysg.waitlink = nil
	gp.waiting = mysg
	mysg.g = gp
	mysg.isSelect = false
	mysg.c = c
	gp.param = nil
	c.recvq.enqueue(mysg)
	// Signal to anyone trying to shrink our stack that we're about
	// to park on a channel. The window between when this G's status
	// changes and when we set gp.activeStackChans is not safe for
	// stack shrinking.
	atomic.Store8(&gp.parkingOnChan, 1)
	gopark(chanparkcommit, unsafe.Pointer(&c.lock), waitReasonChanReceive, traceEvGoBlockRecv, 2)

	// someone woke us up
	if mysg != gp.waiting {
		throw("G waiting list is corrupted")
	}
	gp.waiting = nil
	gp.activeStackChans = false
	if mysg.releasetime > 0 {
		blockevent(mysg.releasetime-t0, 2)
	}
	closed := gp.param == nil
	gp.param = nil
	mysg.c = nil
	releaseSudog(mysg)
	return true, !closed
}
```


### 关闭管道
&emsp;编译器会把`close` 关键字转换为`OCLOSE`以及`runtime.closechan`函数
&emsp;当channel是一个空指针或者已经被关闭了,go会直接panic
```go
func closechan(c *hchan) {
	if c == nil {
		panic(plainError("close of nil channel"))
	}

	lock(&c.lock)
	if c.closed != 0 {
		unlock(&c.lock)
		panic(plainError("close of closed channel"))
	}
}
```
&emsp;上面两种特殊情况, 正常的关闭流程如下:
```go
func closechan(c *hchan) {
	c.closed = 1

	var glist gList

    // 释放所有的readers
	// release all readers
    // 遍历所有的recvq 队列(如果sudog存在元素，则清理内存)
	for {
		sg := c.recvq.dequeue()  // 
		if sg == nil {
			break
		}
		if sg.elem != nil {
			typedmemclr(c.elemtype, sg.elem)
			sg.elem = nil
		}
		if sg.releasetime != 0 {
			sg.releasetime = cputicks()
		}
		gp := sg.g
		gp.param = nil
		if raceenabled {
			raceacquireg(gp, c.raceaddr())
		}
		glist.push(gp)
	}

	// release all writers (they will panic)
	for {
		sg := c.sendq.dequeue()
		if sg == nil {
			break
		}
		sg.elem = nil
		if sg.releasetime != 0 {
			sg.releasetime = cputicks()
		}
		gp := sg.g
		gp.param = nil
		if raceenabled {
			raceacquireg(gp, c.raceaddr())
		}
		glist.push(gp)
	}
	unlock(&c.lock)

	// Ready all Gs now that we've dropped the channel lock.
    // 调度所有的go
	for !glist.empty() {
		gp := glist.pop()
		gp.schedlink = 0
		goready(gp, 3)
	}
}
```
