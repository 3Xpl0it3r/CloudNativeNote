&emsp;go的select和操作系统的select比较类似.c语言里面的select系统调用可以通过监听多个文件描述符的可读可写的状态,go语言的select可以让多个goroutine同时等待多个channel的可读可写的状态.select会一直阻塞当前的线程或者goroutine.

&emsp;select和switch相似的控制结构,select中的case必须都是channel的收发操作.

## 现象
&emsp;当在go里面使用select的时候,会有两个现象:
- select 能够在channel上进行非阻塞的收发操作
- select 在遇到多个channel同时响应的时候,会随机执行一种情况.

#### 非阻塞的收发
&emsp;一般情况下select会阻塞当前的goroutine并且等待多个channel中的一个达到可收发状态.但是如果select控制结构中包含default语句,那么这个select语句在执行时会遇到两种情况:
- 当存在可以收发的channel,直接该channel对应的case
- 当不存在可以收发的channel的时候,执行default里面语句

```go
func main() {
    ch := make(chan int)
    select{
    case i := <- ch:
        println(i)
    default:
        println("this is default")
    }
}
// 运行结果是 "this is default"
```
&emsp;非阻塞的Chanel收发还是很有必要的,很多场景下我们并不希望channel阻塞当前的操作,只想看看channel的可读可写的状态,如下所示:
```go 
errCh := make(chan error, len(tasks))
wg := sync.WaitGroup{}
wg.Add(len(tasks))
for i := range tasks {
    go func() {
        defer wg.Done()
        if err := tasks[i].Run(); err != nil {
            errCh <- err
        }
    }()
}
wg.Wait()

select {
case err := <-errCh:
    return err
default:
    return nil
}
```
&emsp;上么例子里面我们不关心到底成功了多少,只关心有多少的错误,与收发有关的操作:
- `select default`
- `x,ok := <- ch`代替`closed(x)` 来检查状态

#### 随机执行
&emsp;另外一个使用`select` 遇到的情况是同时多个就绪时,select会选择哪个case问题:(是随机选择一个)

## case的数据结构
&emsp;select 在go里面并不存在具体的结构体,但是case有具体的结构体,用`runtime.scase`来表示select控制里面的case.

```go 
type scase struct {
	c           *hchan         // chan
	elem        unsafe.Pointer // data element
	kind        uint16
	pc          uintptr // race pc (for race detector / msan)
	releasetime int64
}
```

## 实现原理
&emsp;`select`在编译期间会被转换为`OSELECT`节点.每个`OSELECT`都会持有一组`OCASE`节点,如果`OCASE`节点为空,那么就意味着它只有一个default节点.
```txt
                    GOLANG OSELECT & OCASE
                                      ┌───────┐
                           ┌─────────►│ OCASE │
                           │          └───────┘
              ┌────────┐   │          ┌───────┐
              │ OSELECT│───┼─────────►│ OCASE │
              └────────┘   │          └───────┘
                           │          ┌───────┐
                           └─────────►│ OCASE │
                                      └───────┘

```

&emsp;编译器在中间代码生产期间会根据`select`中的`case`的不同对控制语句进行优化(优化代码`cmd/compile/internal/gc.walkselectcases`)(优化主要针对以下四种场景):
- select中不存在任何case(也没有default)
- select中只存在一个case
- select中存在两个case,其中一个是default
- select中存在多个case
&emsp;编译器会根据上面四种case分别做一些代码重写和优化

### 直接阻塞
&emsp;当select中不包含任何case
```go 
func walkselectcases(cases *Nodes) []*Node {
	n := cases.Len()
	sellineno := lineno

	// optimization: zero-case select, 当case的个数为0的时候, 直接把select/case语句转换成 对`block`函数的调用
	if n == 0 {
		return []*Node{mkcall("block", nil, nil)}
	}
}
func block() {
	gopark(nil, nil, waitReasonSelectNoCases, traceEvGoStop, 1) // forever(这是涉及到调度)
}
```
&emsp;空的select会阻塞当前的线程,并且当当前goroutine进入永远无法被唤醒的状态;

### 单一管道
&emsp;如果当前select只包含一个case,那么编译器会将select改成写if语句:
```go 
select{
    case v,ok  <- ch: // case ch <- v:
        ....
}
// -> 
if ch == nil {
    block()
}
v,ok := <- ch // case ch <-v
```
&emsp;单一管道时,会根据channel的收发情况生成不同的语句.当case中的channel是空指针的时,会直接挂起当前goroutine,并且永久陷入休眠状态.

### 非阻塞操作
&emsp;当select中仅包含两个case,并且其中有一个是default的时候,go编译器会认为这是一次非阻塞的收发操作,`walkselectcases`会对这周情况做单独的处理,不过在正式优化之前该函数会将case 中所有的channel都转换成指向channel的地址.下面针对非阻塞发送和非阻塞接收分别查看代码;

#### 阻塞发送
&emsp;当case类型是OSEND时,编译器会使用条件语句和`runtime.selectnbsend`函数改写代码
```go 
func walkselectcases(cases *Nodes) []*Node {
	n := cases.Len()
	sellineno := lineno
	// optimization: two-case select but one is default: single non-blocking op.    
	if n == 2 && (cases.First().Left == nil || cases.Second().Left == nil) {    // 
		case OSEND:         // 发送情况     // case <- ch
			// if selectnbsend(c, v) { body } else { default body }
			ch := n.Left
			r.Left = mkcall1(chanfn("selectnbsend", 2, ch.Type), types.Types[TBOOL], &r.Ninit, ch, n.Right)

		case OSELRECV:  // 接收情况
			r.Left = mkcall1(chanfn("selectnbrecv", 2, ch.Type), types.Types[TBOOL], &r.Ninit, elem, ch)
            // r.Left = selectnbrecv(2, ch.Type)

		case OSELRECV2:         // 接收情况
			r.Left = mkcall1(chanfn("selectnbrecv2", 2, ch.Type), types.Types[TBOOL], &r.Ninit, elem, receivedp, ch)
            // r.Left = selectnbrecv2(2, ch.Type)
		}

		r.Left = typecheck(r.Left, ctxExpr)
		r.Nbody.Set(cas.Nbody.Slice())
		r.Rlist.Set(append(dflt.Ninit.Slice(), dflt.Nbody.Slice()...))
		return []*Node{r, nod(OBREAK, nil, nil)}
	}
}
func selectnbsend(c *hchan, elem unsafe.Pointer) (selected bool) {
	return chansend(c, elem, false, getcallerpc())
}
// 
if selectnbsend(ch, i){
    ....
} else {
    ...
}
```
&emsp;`chansend`里面的最后一个参数是false,所以即使不存在接收方或者缓存区不足的时候，当前的goroutine都不会阻塞而是直接返回.

#### 阻塞接收
&emsp;接收和阻塞稍微有点不一样:
```go 

case OSELRECV:  // 接收情况
    r.Left = mkcall1(chanfn("selectnbrecv", 2, ch.Type), types.Types[TBOOL], &r.Ninit, elem, ch)
    // r.Left = selectnbrecv(2, ch.Type)

case OSELRECV2:         // 接收情况
    r.Left = mkcall1(chanfn("selectnbrecv2", 2, ch.Type), types.Types[TBOOL], &r.Ninit, elem, receivedp, ch)
    // r.Left = selectnbrecv2(2, ch.Type)
}

func selectnbrecv(elem unsafe.Pointer, c *hchan) (selected bool) {
	selected, _ = chanrecv(c, elem, false)
	return
}

// compiler implements
//
//	select {
//	case v, ok = <-c:
//		... foo
//	default:
//		... bar
//	}
//
// as
//
//	if c != nil && selectnbrecv2(&v, &ok, c) {
//		... foo
//	} else {
//		... bar
//	}
//
func selectnbrecv2(elem unsafe.Pointer, received *bool, c *hchan) (selected bool) {
	// TODO(khr): just return 2 values from this function, now that it is in Go.
	selected, *received = chanrecv(c, elem, false)
	return
}
```

#### 常见流程
&emsp;多个case的正常逻辑:
```go 
// register cases 注册所有的case
for i, cas := range cases.Slice() {
}
// 
chosen, revcOK := selectgo(selv, order, 3)
if chosen == 0 {
    ...
    break
}
if chosen == 1 {
    ...
    break
}
if chosen == 2 {
    ...
    break
}
```
&emsp;主要的函数是selectgo,这个函数用来选择执行case条件,它一般分为两部分:
- 执行初始化并且确定case的执行顺序:
- 在循环里面根据case类型做出不通的处理
```go 
```

### 初始化
&emsp;`runtime.selectgo`函数首先会进行必要的初始化操作并且决定case的两个顺序-轮训(`pollorder`)和枷锁顺序(`lockorder`);
```go 
func selectgo(cas0 *scase, order0 *uint16, ncases int) (int, bool) {
	cas1 := (*[1 << 16]scase)(unsafe.Pointer(cas0))
	order1 := (*[1 << 17]uint16)(unsafe.Pointer(order0))

	scases := cas1[:ncases:ncases]
	pollorder := order1[:ncases:ncases]
	lockorder := order1[ncases:][:ncases:ncases]
	// Replace send/receive cases involving nil channels with caseNil so logic below can assume non-nil channel.
    for i := range scases {
		cas := &scases[i]
		if cas.c == nil && cas.kind != caseDefault { *cas = scase{} }
    }

    // 轮训排序,  轮训ncases, 引入随机数,随机打乱数组里面的顺序
	for i := 1; i < ncases; i++ {
		j := fastrandn(uint32(i + 1))
		pollorder[i] = pollorder[j]
		pollorder[j] = uint16(i)
	}

    // 按照case里面channle的地址来排序,时间复杂度O(n*log^n)
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

    // 加锁, 按照之前生成的加锁顺序，来锁定select里面包含所有的channel
	sellock(scases, lockorder)
}
// 加锁, 按照之前生成的加锁顺序，来锁定select里面包含所有的channel
func sellock(scases []scase, lockorder []uint16) {
	var c *hchan
	for _, o := range lockorder {
		c0 := scases[o].c
		if c0 != nil && c0 != c {
			c = c0
			lock(&c.lock)
		}
	}
}
```
&emsp;轮训顺序`pollorder`和加锁顺序`lockorder`分别通过下面的方式确认:
- 轮训顺序: 通过`runtime.fastrandn` 函数引入随机性
> 随机轮训避免channel饥饿问题,来保证公平性
- 加锁顺序: 按照channel的地址排序后确定加锁顺序
> 按照地址排序,避免死锁发生,最后调用`runtime.sellock`会按照之前的顺序锁定select语句中包含所有的channel.


### 循环逻辑
&emsp;当select所有的channel被锁定了之后就会进入`runtime.selectgo`主体循环的函数了,它会分三个阶段查找或者等待某个channel就绪:
1. 查找是否已经就绪的channel,即可以执行收发操作
2. 将当前的goroutine加入对应的收发队列上,并且等待其他的goroutine唤醒
3. 当前goroutine被唤醒后,找到满足条件的channel并进行处理

&emsp;`runtime.selectgo`函数会根据不同的情况通过goto语句跳转到函数内部不同的标签执行相应的逻辑:
- `bufrecv`: 可以从缓冲区读取数据
- `bufsend`: 可以向缓冲区写入数据
- `recv`: 可以从休眠的发送方获取数据
- `send`: 可以向休眠的接收方发送数据
- `rclose`: 可以从关闭的Chanel里面读取EOF
- `sclose`: 向关闭的channel里面发送数据
- `retc`: 结束调用并且返回


#### 循环的第一个阶段:(查找就绪的channel)


#### 循环的第二个阶段:(按照需要将当前的goroutine将入到recv队列或者sendq里面)


#### 




