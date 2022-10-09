&emsp;在go里面接口也是一种类型,主要有两种:
- iface
> 带一组方法的接口
- eface
> 不带任何方法的接口

> `go的interface也是一个独立的类型,和c的void*不一样`

## 指针和接口
&emsp;实现接口可以选择指针作为接收者
```go
type Cat struct{}
type Duck interface{}

func(c Cat) Quack{} // 结构体作为实现接口
func(c *Cat) Quack{}    //  使用指针作为接收者

var d Duck = Cat{}  // 使用结构体初始化变量
var d Duck = &Cat{} // 使用结构体指针初始化变量
```
&emsp;规则如下:
```txt
||结构体实现接口|结构体指针实现接口|
|----|----|----|
|结构体初始化变量|通过|不通过|
|结构体指针初始化变量|通过|不通过|
```

*case1*接受者是结构体,初始化变量是指针:
```go
type Duck interface{Quack{}}
type Cat struct{}
func(c Cat)Quack(){
    // todo
}
func main(){
    var c Duck = &Cat{}
    c.Quack()
}
// 因为&Cat{} 能够隐式的获取到指向的结构体,所以能在结构体上调用Quack方法
/*
func(c Cat)Quack(){} --> func Quack(Cat) {}
c.Quack => (*c).Quack
copy一个指针，对指针接引用获取到的结构体还是之前那个，所以ok
*/
```

*case2*接受者是指针,初始化变量是结构体
```go
type Duck interface{Quack{}}
type Cat struct{}
func(c *Cat)Quack(){
    // todo
}
func main(){
    var c Duck = Cat{}
    c.Quack()
}
/*
func(c *Cat)Quack(){} --> func Quack(*Cat) {}
对于这个case来讲,先copy了一个结构体，这对这个结构体取地址，但是这个结构体已经不是先前的结构体了
*/
```

## `Nil`和`none-nil`
&emsp; 
```go
type TestStruct struct { }
func NilOrNot(v interface{})bool{
    return v== nil
}
func main() {
    var s  *TestStruct
    fmt.Println(s == nil)
    fmt.Println(NilOrNot(s))
}
/*
true
false
NilOrNot 发生了隐式的转换
*/
```

## 数据结构
&emsp;go里面根据接口类型是否包含一组方法将接口分类:
- `runtime.iface` 包含方法的接口
- `runtime.eface` 不包含方法的接口

```go
type eface struct {
	_type *_type                        // 指向类型结构体的指针
	data  unsafe.Pointer                // 指向底层数据的指针
}


type iface struct {
	tab  *itab                          // runtime.itab字段(类似虚函数表)
	data unsafe.Pointer                 // 指向原始数据的指针
}
```

### 类型结构体
&emsp;`runtime._type`是go语言运行时的表示.
```go
type _type struct {
	size       uintptr                      // 字段存储了类型占用的内存空间
	ptrdata    uintptr // size of memory prefix holding all pointers
	hash       uint32                      //  用来判断类型是不是同一个类型
	tflag      tflag 
	align      uint8
	fieldAlign uint8
	kind       uint8
	// function for comparing objects of this type
	// (ptr to object A, ptr to object B) -> ==?
	equal func(unsafe.Pointer, unsafe.Pointer) bool             //用来判断当前类型的多个对象是不是相等
	// gcdata stores the GC type data for the garbage collector.
	// If the KindGCProg bit is set in kind, gcdata is a GC program.
	// Otherwise it is a ptrmask bitmap. See mbitmap.go for details.
	gcdata    *byte
	str       nameOff
	ptrToThis typeOff
}
```

### itab结构体
&emsp;`runtime.itab`是接口的核心的组成部分,占据32个字节
```go
type itab struct {
	inter *interfacetype
	_type *_type
	hash  uint32 // copy of _type.hash. Used for type switches. 当想要把intetface转换成具体类型的时候，通过对比hash来判断是不是原来的类型
	_     [4]byte
	fun   [1]uintptr // variable sized. fun[0]==0 means _type does not implement inter., 动态大小的数组,里面存储了一组函数的指针(类似动态派发的虚函数表)
}
```

## 类型转换

### 指针类型
```go
type Duck interface {
	Quack()
}
type Cat struct {
	Name string
}
//go:noinline
func (c *Cat) Quack() {
	println(c.Name + " meow")
}
func main() {
	var c Duck = &Cat{Name: "draven"}
	c.Quack()
}
```
&emsp;汇编后的代码如下(Cat初始化部分):
```go
"".main STEXT size=177 args=0x0 locals=0x38
	0x0013 00019 (main.go:14)	SUBQ	$56, SP                                                 //开辟栈空间
	0x0017 00023 (main.go:14)	MOVQ	BP, 48(SP)
	0x001c 00028 (main.go:14)	LEAQ	48(SP), BP                                              // 保存callee的栈指针
	0x0021 00033 (main.go:15)	LEAQ	type."".Cat(SB), AX                                     // AX = &type."".Cat
	0x0028 00040 (main.go:15)	MOVQ	AX, (SP)                                                // SP = &type."".Cat
	0x002c 00044 (main.go:15)	CALL	runtime.newobject(SB)                                   // SP + 8 = &Cat{}
	0x0031 00049 (main.go:15)	MOVQ	8(SP), DI                                               // DI = &Cat{}
	0x0036 00054 (main.go:15)	MOVQ	DI, ""..autotmp_2+16(SP)                                // SP + 16 = &Cat{}
	0x003b 00059 (main.go:15)	MOVQ	$6, 8(DI)                                               // StringHeader{DI.Name}.len = 6
	0x004e 00078 (main.go:15)	LEAQ	go.string."draven"(SB), AX                              // AX = &"draven"
	0x0055 00085 (main.go:15)	MOVQ	AX, (DI)                                                // StringHeader{DI.Name}.Data = &"draven"
```
&emsp;初始化步骤如下:
- 先将`Cat`这个结构体的类型指针存放到栈上(作为`runtime.newobject`的参数)
- `runtime.newobject` 创建一个`Cat`对象,并且把这个对象的地址作为返回值返回到栈上(参数栈+8)(SP+8)
- `SP+8`存放`&Cat`,方便操作先将`Cat`的地址copy到`DI`寄存器
- `Cat`只有一个`Name`的对象(`StringHeader`结构),会将字符串的长度以及字符串的地址分别copy到`StringHeader`这个结构体里面
Cat结构体在栈上结构
```txt
                    Struct Cat on Stack

                                        ┌───────┐
                ┌────────────┐          │   8   │
                │   *Cat     │────────►┌─────────┐ 
                └────────────┘         │Helloword│
                                       └─────────┘
                                         "heap"
                top of stack
```

&emsp;转换过程,由于`Cat`里面就一个string(16个字节),因此`Cat`大小也是16字节
```go
    0x005a 00090 (main.go:15)	MOVQ	""..autotmp_2+16(SP), AX            ;; AX = &Cat{Name: ""}
	0x005f 00095 (main.go:15)	MOVQ	AX, ""..autotmp_1+24(SP)            ;; SP = AX
	0x0064 00100 (main.go:15)	LEAQ	go.itab.*"".Cat,"".Duck(SB), CX     ;; CX = *itab(go.itab.*"".Cat,"".Duck)
	0x006b 00107 (main.go:15)	MOVQ	CX, "".c+32(SP)                     ;; SP + 32 = CX / *itab(go.itab.*"".Cat,"".Duck)
	0x0070 00112 (main.go:15)	MOVQ	AX, "".c+40(SP)                     ;; SP + 40 = AX / &Cat{Name:"xxx"}
	0x0075 00117 (main.go:16)	MOVQ	"".c+32(SP), AX                     ;; AX = SP + 32 / *itab(go.itab.*"".Cat,"".Duck)
	0x007a 00122 (main.go:16)	TESTB	AL, (AX)                            ;; 
	0x007c 00124 (main.go:16)	MOVQ	24(AX), AX                          ;; AX = *itab + 24 (itab.func)
	0x0080 00128 (main.go:16)	MOVQ	"".c+40(SP), CX                     ;; CX = &Cat{Name:"dsadas"}
	0x0085 00133 (main.go:16)	MOVQ	CX, (SP)                            ;; SP = CX, SP = &Cat{Name:"dasdas"}
	0x0089 00137 (main.go:16)	CALL	AX                                  ;; Call *itab->func
```
&emsp;栈结构如下:
```txt
                                         ┌────────┐
           ┌────────────────────┐        │ 6      │
           │    *Cat            │───────►│        │ 
           └────────────────────┘        │"draven"│
           ┌────────────────────┐        └────────┘
           │  go.itab.*Cat, Duck│                       SP
           └────────────────────┘
           top of stack
```
