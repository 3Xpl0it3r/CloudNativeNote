
### 从内存角度看,并行计算只有两种: 共享内存,消息通信(内存copy)
&emsp;基于共享内存的并发原语基本上都是需要实现互斥锁的同步原语(go里面mutex),在将CSP(基于输入输出编程结构的编程语言)实现为显示的channel同步原语


&emsp;channel的数据结构:
```go
// hchan是channel在runtime里面的表示
type hchan struct {
	qcount   uint           // total data in the queue 队列里面所有数据
	dataqsiz uint           // size of the circular queue , 环形队列的大小
	buf      unsafe.Pointer // points to an array of dataqsiz elements,  指向大小为dataqsiz的数组
	elemsize uint16         // 元素大小
	closed   uint32         // channel是否已经被关闭
	elemtype *_type // element type         // 元素的类型
	sendx    uint   // send index(发送索引)
	recvx    uint   // receive index(接收索引)
	recvq    waitq  // list of recv waiters(recvice等待列表 [g1, g2, g3]  <- ch)
	sendq    waitq  // list of send waiters(send 等待列表  ch <- [g1, g2, g3])

	lock mutex
}
// 等待队列
type waitq struct {     // 等待队列sudog 双向队列
	first *sudog
	last  *sudog
}
type sudog struct {
	g *g
    // 表示当前g是否正在参与select的操作
	isSelect bool
}
```

## 实现原理
&emsp;`select`在编译期间会被转换成`OSELECT` 节点.每个节点`OSELECT`都会持有一组`OCASE`节点,如果`OCASE`的执行条件为空,这意味着它是一个default节点
`OSELECT`会转换成`OCASE`, 
&emsp;编译器会在中间代码生成`select`中的case语句做优化,主要会分为下面四种情况:
- select 不包含任何的case
- select 只存在一个case 
- select 存在两个case,其中一个case是default
- select 存在多个case


#### 只有一个default的情况下
&emsp;编译器在AST阶段就直接把它优化成单个语句

#### 0个isSelect
&emsp; 不包含任何case的情况下:
```go
func walkselectcases(cases *Nodes) []*Node {
	n := cases.Len()
	if n == 0 {
		return []*Node{mkcall("block", nil, nil)}
	}
}
func block() {
    // 将当前的goroutine 防止到全局等待队列里面(永远不会被调用到)
	gopark(nil, nil, waitReasonSelectNoCases, traceEvGoStop, 1) // forever
}
```

#### 只有一个case情况下
&emsp;只有一个case那就意味着是单一发送/单一接收(如果是default直接报错)
```go
func walkselectcases(cases *Nodes) []*Node {
		cas := cases.First()
		setlineno(cas)
		l := cas.Ninit.Slice()
		if cas.Left != nil { // not default:  //  不是default语句
			switch n.Op {
			default:    // 语法错误
				Fatalf("select %v", n.Op)
				// ok already
            cas OSEND:         // case ch <- ch:
				ch = n.Left
			case OSELRECV, OSELRECV2:
				// 新建as节点
			}
            //如果ch为空指针，直接转换成block 
			a.Left = nod(OEQ, ch, nodnil())
			var ln Nodes
			ln.Set(l)
			a.Nbody.Set1(mkcall("block", nil, &ln))
            //  转换成if节点
			a := nod(OIF, nil, nil)
		}
}
```
&emsp;从代码可以看出单一case节点情况下会直接被转换成if节点,example如下:
```txt
// 单一接收
// 原始代码:
select {
        case k <- ch:
            //todo
}
// 转换后
if k<- ch {
    //todo
}

// k,v接收
// 原始代码
select {
    case k,ok <- ch:
        //todo
}
// 转换后的代码
if k, ok <- ch {
    // todo
}

// 发送
// 原始代码
select {
    case ch <- data:
        // todo
}
// 转换后的代码
if ch <- data{
    // todo
}
```

#### 非阻塞操作
&emsp;当有两个case,并且其中一个是default语句(这种就是非阻塞)go编译器就会认为这是一次非阻塞的操作,`walkselectcase`会对这种情况做单独的处理,在优化之前会把case里面的所有的channel转换成指向channel的地址(通过typecheck ctxExpr操作)
```go
	for _, cas := range cases.Slice() {
		setlineno(cas)
		n := cas.Left
		if n == nil {
			continue
		}
		switch n.Op {
	case OSEND:
			n.Right = nod(OADDR, n.Right, nil)
			n.Right = typecheck(n.Right, ctxExpr)

		case OSELRECV, OSELRECV2:
			if n.Op == OSELRECV2 && n.List.Len() == 0 {
				n.Op = OSELRECV
			}

			if n.Left != nil {
				n.Left = nod(OADDR, n.Left, nil)
				n.Left = typecheck(n.Left, ctxExpr)
			}
		}
	}
```
&emsp;针对只有两个节点的优化如下:
```
	// optimization: two-case select but one is default: single non-blocking op.
	if n == 2 && (cases.First().Left == nil || cases.Second().Left == nil) {
		var cas *Node
		var dflt *Node
        // 如果第一个case是nil,那么第一个case会被设置成default语句
		if cases.First().Left == nil {
			cas = cases.Second()
			dflt = cases.First()
		} else {
			dflt = cases.Second()
			cas = cases.First()
		}

		n := cas.Left
		setlineno(n)
        // 转换成if语句
		r := nod(OIF, nil, nil)
		r.Ninit.Set(cas.Ninit.Slice())
		switch n.Op {
		default:
			Fatalf("select %v", n.Op)

		case OSEND:
            // 如果收发送操作 (ch <- data ) -->  优化成selectnbsend
			// if selectnbsend(c, v) { body } else { default body }
			r.Left = mkcall1(chanfn("selectnbsend", 2, ch.Type), types.Types[TBOOL], &r.Ninit, ch, n.Right)

		case OSELRECV:
            // 接收操作1(只有一个操作元素  data <- ch) , 优化成selectnbrecv
			// if selectnbrecv(&v, c) { body } else { default body }
			r.Left = mkcall1(chanfn("selectnbrecv", 2, ch.Type), types.Types[TBOOL], &r.Ninit, elem, ch)

		case OSELRECV2:
			// if selectnbrecv2(&v, &received, c) { body } else { default body }
			r = nod(OIF, nil, nil)
			r.Left = mkcall1(chanfn("selectnbrecv2", 2, ch.Type), types.Types[TBOOL], &r.Ninit, elem, receivedp, ch)
		}
	}
```

#### 非阻塞的发送
&emsp; 发送会被优化成`selectnbsend`,并且当channel已经被关闭或者缓冲区已经满了的情况下直接返回false
```go 
func selectnbsend(c *hchan, elem unsafe.Pointer) (selected bool) {
    // 非阻塞的发送一个channel
	return chansend(c, elem, false, getcallerpc())
}
func chansend(c *hchan, ep unsafe.Pointer, block bool, callerpc uintptr) bool {
    // 也就意味着它在被关闭或者缓冲区已经满了的情况下不会阻塞直接return false
	if !block && c.closed == 0 && full(c) {
		return false
	}
}
```

#### 非阻塞的接收情况
&emsp;非阻塞的接收会根据接收对象是1个还是2个会被优化成`selectnbrecv`和`selectnbrecv2`两个函数
&emsp;接收对象只有一个,并当channel为空或者channel已经被关闭了情况下,也是直接返回false
```go 
func selectnbrecv(elem unsafe.Pointer, c *hchan) (selected bool) {
    // 非阻塞的从channel里面接收数据, block参数设置了false
	selected, _ = chanrecv(c, elem, false)
	return
}

func chanrecv(c *hchan, ep unsafe.Pointer, block bool) (selected, received bool) {
    // 非阻塞状态下,如果channel为空，直接返回
	if c == nil {
		if !block { return }
	}
	if !block && empty(c) { // 非阻塞为空
		if atomic.Load(&c.closed) == 0 { return }   // 如果channel已经关闭了,直接return
		if empty(c) { return true, false }          // 如果channel为空, 直接return true, false
}
```
&ems;`selectnbrecv2`的情况和`selectnbrecv`情况基本查不多
```go 
func selectnbrecv2(elem unsafe.Pointer, received *bool, c *hchan) (selected bool) {
	// TODO(khr): just return 2 values from this function, now that it is in Go.
	selected, *received = chanrecv(c, elem, false) // 唯一的差别就是received这个值被返回了
	return
}
```

#### 其他情况
&emsp;除了上面几种情况,下面分析下正常的select的情况,在一般情况下,编译器会做如下的操作:
1. 将所有的case转化成包含Channel以及类型等信息的`runtime.scase`结构体
2. 调用`runtime.selectgo` 从多个准备就绪的channel里面选择一个可以执行的`runtime.scase`结构体 ----- 随机化case, select本质上就是一个随机化的if(普通if每次的执行顺序都是一致的,但是selectgo的顺序是随机的)
3. 通过for生成一组if语句,在语句里面判断自己是不是被选中的case 

```go 
func walkselectcases(cases *Nodes) []*Node {
	var init []*Node
	// generate sel-struct
    // 生成两个节点 selv 节点和 order节点
	selv := temp(types.NewArray(scasetype(), int64(n)))
	init = append(init, r)

	order := temp(types.NewArray(types.Types[TUINT16], 2*int64(n)))
	init = append(init, r)

	// register cases
    // 注册所有的case语句
	for i, cas := range cases.Slice() {
        init = append(init ,xxx)
	}

	// run the select
    //  开始调用selectgo 来运行selectgo函数了
	fn := syslook("selectgo")


	// dispatch cases
	for i, cas := range cases.Slice() {
	}

	return init
}
```
> 总结: 普通情况下会把所有的case给收集起来，然后在selectgo函数里面随机选择一个执行

#### selectgo 函数
&emsp;这步骤是分支随机化的过程,在selectgo里面首先会执行一些必要的初始化的动作,然后在对case做一些排序(轮训排序pollorder和加锁排序lockorder)
```go 
func selectgo(cas0 *scase, order0 *uint16, ncases int) (int, bool) {
    // 先将channel为nil的case都用caseNil 来代替,因此下面的逻辑我们就可以假设没有空channel了
	for i := range scases {
		cas := &scases[i]
		if cas.c == nil && cas.kind != caseDefault { *cas = scase{} }
	}
    // 生成一个随机的顺序,通过fastrandn来生成随机索引值
	// generate permuted order
	for i := 1; i < ncases; i++ {
		j := fastrandn(uint32(i + 1))
		pollorder[i] = pollorder[j]
		pollorder[j] = uint16(i)
	}

	// sort the cases by Hchan address to get the locking order.
	// simple heap sort, to guarantee n log n time and constant stack footprint.
    // 按照channel的地址大小排序,时间复杂度nlongn, 这个用来确定加锁的顺序
	for i := 0; i < ncases; i++ {
		j := i
		// Start with the pollorder to permute cases on the same channel.
		c := scases[pollorder[i]].c
		for j > 0 && scases[lockorder[(j-1)/2]].c.sortkey() < c.sortkey() {
			k := (j - 1) / 2
			lockorder[j] = lockorder[k]
			j = k
		}
		lockorder[j] = pollorder[i]
	}
	for i := ncases - 1; i >= 0; i-- {
		o := lockorder[i]
		c := scases[o].c
		lockorder[i] = lockorder[0]
		j := 0
		for {
			k := j*2 + 1
			if k >= i {
				break
			}
			if k+1 < i && scases[lockorder[k]].c.sortkey() < scases[lockorder[k+1]].c.sortkey() {
				k++
			}
			if c.sortkey() < scases[lockorder[k]].c.sortkey() {
				lockorder[j] = lockorder[k]
				j = k
				continue
			}
			break
		}
		lockorder[j] = o
	}

    // 依次对locker里面的进行加锁
	sellock(scases, lockorder)
}
```

##### selectgo 循环阶段
&emsp; 循环阶段是寻找一个合适的case分支来执行(查找会分为三个阶段来查找，任意一个阶段查到了就会执行对应的分支,查找不到继续循环)
-  pass 1 - look for something already waiting 查找是否已经存在准备就绪的channel,(即可以立刻执行收发操作 )
-  pass 2 - enqueue on all chans (将当前的goroutine加入到channel的收发队列上面,并且等待其他goroutine来换醒)
-  pass 3 - dequeue from unsuccessful chans(当前goroutine被唤醒了,找到满足条件的channel来执行)

```go 
	for i := 0; i < ncases; i++ {
		casi = int(pollorder[i])
		cas = &scases[casi]
		c = cas.c

		switch cas.kind {
		case caseNil: //当cas不包含channel的时候直接跳过
			continue

		case caseRecv:                  // 如果是ORECV操作
			sg = c.sendq.dequeue()      // 从发送队列里面找到sg, 如果存在sg,则执行recv操作, 阻塞操作, 无buffer的channel
			if sg != nil {
				goto recv
			}
			if c.qcount > 0 {           // 如果缓冲区里面存在元素，执行bufrecv操作 // 非阻塞操作/存在buffer的情况
				goto bufrecv
			}
			if c.closed != 0 {          // 如果channel已经被关闭了，执行rclose操作
				goto rclose
			}

		case caseSend:                  //  是Send的操作
			if raceenabled {
				racereadpc(c.raceaddr(), cas.pc, chansendpc)
			}
			if c.closed != 0 {       //  如果channel已经被关闭了, 执行sclose操作
				goto sclose
			}
			sg = c.recvq.dequeue()      // 从接收队列里面获取sg, 如果存在执行send操作
			if sg != nil {
				goto send
			}
			if c.qcount < c.dataqsiz {  // buffer里还没有满, 则执行bufsend操作
				goto bufsend
			}

		case caseDefault:           // default语句
			intdfli = casi
			dfl = cas
		}
	}

    // 如果存在default分支，则直接去执行default分支
	if dfl != nil {
		selunlock(scases, lockorder)
		casi = dfli
		cas = dfl
		goto retc
	}
```

##### stage2 将所有的channel加入到收发队列里面
```go 
// 将所有的channel封装成sudog，然后加入到channel的收发队列里面
	gp = getg()     // 获取当前的goroutine
	nextp = &gp.waiting
	for _, casei := range lockorder {
		c = cas.c
		sg := acquireSudog()
		sg.g = gp
		sg.isSelect = true
		sg.elem = cas.elem
		sg.c = c
		switch cas.kind {
		case caseRecv:
			c.recvq.enqueue(sg)
		case caseSend:
			c.sendq.enqueue(sg)
		}
	}
    // gopark将当前grouting陷入休眠状态，等待被唤醒
	gopark(selparkcommit, nil, waitReasonSelect, traceEvGoBlockSelect, 1)
    // 重新上锁
	sellock(scases, lockorder)
```

##### stage3 将所有channel出队列
```go 
	casi = -1
	cas = nil
	sglist = gp.waiting
	// Clear all elem before unlinking from gp.waiting.
    // 在gp.waiting解除链接之前清理掉sudo上所有的elem
	for sg1 := gp.waiting; sg1 != nil; sg1 = sg1.waitlink {
		sg1.isSelect = false
		sg1.elem = nil
		sg1.c = nil
	}
	gp.waiting = nil

    // 遍历所有的case，找到当前已经被唤醒的case，其他的case则会被从收发队列里面被移除掉
	for _, casei := range lockorder {
		k = &scases[casei]
		if k.kind == caseNil {
			continue
		}
		if sg == sglist {
			// sg has already been dequeued by the G that woke us up.
			casi = int(casei)
			cas = k
		} else {
			c = k.c
			if k.kind == caseSend {
				c.sendq.dequeueSudoG(sglist)
			} else {
				c.recvq.dequeueSudoG(sglist)
			}
		}
		releaseSudog(sglist) //释放sudog
		sglist = sgnext
	}

    // 如果没有找到，则继续循环
	if cas == nil {
		goto loop
	}

	c = cas.c

	if debugSelect {
		print("wait-return: cas0=", cas0, " c=", c, " cas=", cas, " kind=", cas.kind, "\n")
	}

	if cas.kind == caseRecv {
		recvOK = true
	}

	if raceenabled {
		if cas.kind == caseRecv && cas.elem != nil {
			raceWriteObjectPC(c.elemtype, cas.elem, cas.pc, chanrecvpc)
		} else if cas.kind == caseSend {
			raceReadObjectPC(c.elemtype, cas.elem, cas.pc, chansendpc)
		}
	}
	if msanenabled {
		if cas.kind == caseRecv && cas.elem != nil {
			msanwrite(cas.elem, c.elemtype.size)
		} else if cas.kind == caseSend {
			msanread(cas.elem, c.elemtype.size)
		}
	}

	selunlock(scases, lockorder)
	goto retc
```


#### 分送分支代码详解

##### bufrecv(无阻塞接收,带有缓冲区的channel)
```go 
&emsp;这部分逻辑和channel的buferrecv一样，没啥区别
bufrecv:
	recvOK = true
	qp = chanbuf(c, c.recvx)   // 获取存放取数据的地址
	if cas.elem != nil {        // 如果接收地址存在，直接通通过typdmmove将channel缓冲区对应的数据直接copy到接收地址对应的内存快
		typedmemmove(c.elemtype, cas.elem, qp)
	}
	typedmemclr(c.elemtype, qp)     // 清理缓冲期已经被取出的数据
	c.recvx++                       // 接收buffer索引+1

	if c.recvx == c.dataqsiz {
		c.recvx = 0
	}
	c.qcount--
	selunlock(scases, lockorder)
	goto retc
```
##### bufsend(无阻塞的发送,带有缓冲区的channel)
&emsp;逻辑和bufsend一样
```go 
bufsend:
	typedmemmove(c.elemtype, chanbuf(c, c.sendx), cas.elem)
	c.sendx++
	if c.sendx == c.dataqsiz {
		c.sendx = 0
	}
	c.qcount++
	selunlock(scases, lockorder)
```
##### recv(阻塞接收)
```go 
recv:
	// can receive from sleeping sender (sg)
    // 通过recv函数来从sg里面接收数据,接收完成调用selunlock 来解锁所有的scases
	recv(c, sg, cas.elem, func() { selunlock(scases, lockorder) }, 2)
	if debugSelect {
		print("syncrecv: cas0=", cas0, " c=", c, "\n")
	}
	recvOK = true
	goto retc

//- --------
func selunlock(scases []scase, lockorder []uint16) {
	for i := len(scases) - 1; i >= 0; i-- {
		c := scases[lockorder[i]].c
		unlock(&c.lock)
	}
}
```

#### rclose(接收管道已经被关闭了)
```go 
rclose:
	// read at end of closed channel
    // 直接先解锁所有的channel
	selunlock(scases, lockorder)
	recvok = false
    //  如果接收地址存在，直接清理掉接收地址所在的内存空间
	if cas.elem != nil {    typedmemclr(c.elemtype, cas.elem) }
	goto retc

// ------
func typedmemclr(typ *_type, ptr unsafe.Pointer) {
    // memclrNoHeapPointers clears n bytes starting at ptr.
	memclrNoHeapPointers(ptr, typ.size)
}
```

##### send(阻塞发送)
&emsp;调用send
```go 
send:
	// can send to a sleeping receiver (sg)
	send(c, sg, cas.elem, func() { selunlock(scases, lockorder) }, 2)
	goto retc
```

#### retc(返回)
```go 
retc:
	return casi, recvOK
```

##### sclose(返送管道管道)
```go 
sclose:
	selunlock(scases, lockorder)
	panic(plainError("send on closed channel"))
}
//- --------
func selunlock(scases []scase, lockorder []uint16) {
	for i := len(scases) - 1; i >= 0; i-- {
		c := scases[lockorder[i]].c
		unlock(&c.lock)
	}
}
```
