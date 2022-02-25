##### 总结
&emsp;`Context`的设计理念- 在`goroutine`里面构成树形结构对信号同步，减少资源浪费
&emsp; `Context`的是一个接口，只要实现了四个方法`Deadline,Done,Err,Value`
&emsp; 种类
- Background  默认使用的空context, 本质上是一个int类型
- TODO        空的context，不知道应该用啥的时候可以用todo，本质和background是一样的

&emsp;`With`四个函数
- `WithCancel`
>  返回一个带有`Done`channel的Parent Context., 然后通过`propagateCancel(parent, &c)`函数来构建上下文的父子关系。当父上下文被取消的时候，子上下文也会被取消。
- `WithTimeout`
> 对 withDeadline 的一个封装
- `WithDeadline`
> 返回一个deadline时间
- `WithValue`
> 一般情况下很少用




#### 0x00 Introduce
&emsp;`Context`在go1.7以后引入进来的。该接口定义了四个方法：
- `Deadline`- 返回`context.Context`被取消的时间，也就是完成工作的时间
- `Done` - 返回一个Channel， 这个Channel会在当前工作完成后，或者上下文被取消后关闭，多次调用会返回同一个Channel
- `Err` 返回`context.Context` 结束的原因，它只会在`Done`方法对应的Channel里面被关闭时候返回一个非空的值
- * 如果`context.Context`被取消，会返回一个`Canceled`的错误
- * 如果`context.Context`超时了，会返回一个`DeadlineExceed`的错误
- `Value` - 从`context.Context` 中获取键对应的值，对于同一个上下文来说，多次调用`Value`并传入相同的`key`会返回相同的结果，该方法可以用来传递请求特定的数据。


#### 0x01 设计原理
&emsp;在`goroutine`构成的树形结构里面对信号进行同步以减少资源浪费是`context.Context`最大作用。go服务每个请求都通过一个`goroutine`去处理，http/rpc 在处理请求的时候会启动新的`goroutine`去访问数据库和其他的服务。

&emsp;在一个服务里面可能会创建多个goroutine 去处理一个请求，而`context.Context`的作用是在不同的goroutine之间同步请求特定的数据，取消信号，以及处理请求的截止日期。

&emsp;每个`context.Context`都会从最顶层的goroutine一层层的往下传递，`context.Context`可以在上下层goroutine执行错误的时候将信号同步给下层。 当上层因为某些原因失败的时候，下层由于没有接受到这个信号所以会去继续工作，但是当我们正确的使用`context.Context`时候，就可以减少额外的资源消耗。


#### 0x02 默认上下文
&emsp;`context`包中最常用的方法还是`context.Background`和`context.TODO`这两个方法都会返回事先初始化好的私有变量`background`和`todo`

```go 
func Background() Context{
    return background
}
func TODO()Context {
    return todo
}

var (
	background = new(emptyCtx)
	todo       = new(emptyCtx)
)

```
&emsp;`emptyCtx`是一个int类型 ,他实现了Context的方法。他只是实现了空的方法，实际上不具备任何功能。
```go 
type emptyCtx int

func (*emptyCtx) Deadline() (deadline time.Time, ok bool) {
	return
}

func (*emptyCtx) Done() <-chan struct{} {
	return nil
}

func (*emptyCtx) Err() error {
	return nil
}

func (*emptyCtx) Value(key interface{}) interface{} {
	return nil
}

func (e *emptyCtx) String() string {
	switch e {
	case background:
		return "context.Background"
	case todo:
		return "context.TODO"
	}
	return "unknown empty Context"
}
```
&emsp;从源代码来看`context.Background`和`context.TODO` 也只是互为别名，没有太大差别，只是语义上稍微不同而已
-  `context.Background` 是上下文的默认值，其他所有的上下文都应该从这个地方衍生出来
- `context.TODO` 应该仅仅在不确定使用那种上下文时候使用

> 在大多数情况下，如果函数没有上下文作为入参的时候，我们都会使用`context.Background`作为起始上下文向下传递信号

#### 0x03 取消信号
&emsp;`context.WithCancel()`方法能够从`context.Context`中衍生出一个新的上下文并返回一个用于去取消上下文的函数，一旦执行返回的去取消函数，当前上下文以及它的子上下文都会被取消掉。所有goroutine 都会同步收到这个取消的信号

&emsp;我们直接从`context.WithCancel`函数实现看它具体做啥：
```go 
func WithCancel(parent Context) (ctx Context, cancel CancelFunc) {
	if parent == nil {
		panic("cannot create context from nil parent")
	}
	c := newCancelCtx(parent)
	propagateCancel(parent, &c)
	return &c, func() { c.cancel(true, Canceled) }
}

```
&emsp;`newCancelCtx()`将传入的上下文包装上一个新的私有结构体`cancelCtx`，
```go 
// newCancelCtx returns an initialized cancelCtx.
func newCancelCtx(parent Context) cancelCtx {
	return cancelCtx{Context: parent}
}
type cancelCtx struct {
	Context

	mu       sync.Mutex            // protects following fields
	done     atomic.Value          // of chan struct{}, created lazily, closed by first cancel call
	children map[canceler]struct{} // set to nil by the first cancel call
	err      error                 // set to non-nil by the first cancel call
}
```

&emsp;`propagateCancel(parent, &c)`函数构建父子上下文关联关系，当父上下文被取消的时候，子上下文也会被取消
```go 
// propagateCancel arranges for child to be canceled when parent is.
func propagateCancel(parent Context, child canceler) {
	done := parent.Done()
	if done == nil {
		return // parent is never canceled
	}

	select {
	case <-done:
		// parent is already canceled
		child.cancel(false, parent.Err())
		return
	default:
	}

	if p, ok := parentCancelCtx(parent); ok {
		p.mu.Lock()
		if p.err != nil {
			// parent has already been canceled
			child.cancel(false, p.err)
		} else {
			if p.children == nil {
				p.children = make(map[canceler]struct{})
			}
			p.children[child] = struct{}{}
		}
		p.mu.Unlock()
	} else {
		atomic.AddInt32(&goroutines, +1)
		go func() {
			select {
			case <-parent.Done():
				child.cancel(false, parent.Err())
			case <-child.Done():
			}
		}()
	}
}
```


#### 0x04 传值方法
&emsp;传值一般很少用到，一般在传递请求对用户的认证令牌以及用于分布式追踪的请求ID



