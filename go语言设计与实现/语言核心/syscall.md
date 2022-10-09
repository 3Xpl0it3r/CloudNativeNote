&emsp;传值和传引用区别:
- 传值: 函数调用会对参数进行拷贝,被调用方和调用方两者持有不相同的两份数据
- 传引用: 函数调用会传递参数的指针, 被调用方和调用方持有相同的数据,任意一方修改都会影响另外一方
&emsp;在go里面传参是传值,无论是基本类型,结构体还是指针, 都会对参数进行copy.


#### go的调用约定
```go
func MyFunction(a, b int) (int ,int) {
    return a + b , a - b
}
func main() {
    MyFunction(66, 77)
}
```
编译后结果如下:
```asm
"".main STEXT size=71 args=0x0 locals=0x28
    ....
	0x000f 00015 (demo.go:7)	SUBQ	$40, SP             //分配40个字节的栈空间
	0x0013 00019 (demo.go:7)	MOVQ	BP, 32(SP)          // 将BP寄存器放到顶部8个字节
	0x0018 00024 (demo.go:7)	LEAQ	32(SP), BP          // 将当前的栈指针放到BP寄存器里面
	0x001d 00029 (demo.go:8)	MOVQ	$66, (SP)           // 将66 放到栈顶 SP         第一个参数
	0x0025 00037 (demo.go:8)	MOVQ	$77, 8(SP)          // 将77 放到栈SP+8位置      第二个参数
    // 可以看出go的调用约定从左到右
	0x002e 00046 (demo.go:8)	PCDATA	$1, $0
	0x002e 00046 (demo.go:8)	CALL	"".MyFunction(SB)   //调用MyFunction 函数
	0x0033 00051 (demo.go:9)	MOVQ	32(SP), BP          // 恢复BP寄存器
	0x0038 00056 (demo.go:9)	ADDQ	$40, SP             //释放栈空间
	0x003c 00060 (demo.go:9)	RET
```

&emsp;调用图如下
```
                golang调用约定

              ┌───────┐◄─────────── BP
              │ base  │          
              ┌───────┐◄───────────(32)SP
              │       │
              │       │◄───────────(24)SP
              │       │
              ┌───────┐◄────────── (16)SP
              │  77   │
              │       │◄───────────(8)SP
              │  88   │
              └───────┘◄───────────(0)SP
```

####  整型和数组
&emsp;example如下:
```golang
 func MyFunction(i int, arr [2]int)  {
    i = 29
    arr[1] = 33
    fmt.Printf("in my_function - i = (%d, %p)  arr=(%v, %p)\n", i, &i, arr, &arr)
}

func main() {
    i := 30
    arr := [2]int{66,77}
    fmt.Printf("before calling - i = (%d, %p)  arr=(%v, %p)\n", i, &i, arr, &arr)
    MyFunction(i, arr)
    fmt.Printf("after calling - i = (%d, %p)  arr=(%v, %p)\n", i, &i, arr, &arr)
}
```
> go的整数和数组都是值传递,调用函数时会对内容进行copy,因此拷贝大数组会有性能问题

#### 结构体和指针
&emsp;example
```golang
type MyStruct struct{
    i int
}
func MyFunction(a MyStruct, b *MyStruct)  {
    a.i = 20
    b.i = 30
    fmt.Printf("in my_function - i = (%d, %p)  arr=(%v, %p)\n", a, &a, b, &b)
}
func main() {
    a := MyStruct{i: 10}
    b := MyStruct{i: 11}
    fmt.Printf("before calling - i = (%d, %p)  arr=(%v, %p)\n", a, &a, b, &b)
    MyFunction(a, &b)
    fmt.Printf("after calling - i = (%d, %p)  arr=(%v, %p)\n", a, &a, b, &b)
}
/*
before calling - i = ({10}, 0xc0000b2008)  arr=({11}, 0xc0000b2010)
in my_function - i = ({20}, 0xc0000b2018)  arr=(&{30}, 0xc0000ac020)
after calling - i = ({10}, 0xc0000b2008)  arr=({30}, 0xc0000b2010)
*/
```
> 结论:
- 传递结构体时: 会copy结构体里面所有的内容
- 传递结构体指针时: copy 结构体指针
&emsp;修改结构体指针是改变了指针指向的结构体,`b.i`可以看成`(*b).i`, 也就是先获取指针背后的结构体,代码修改如下:
```golang

type MyStruct struct{
    i int
    j int
}
func MyFunction(ms *MyStruct)  {
    ptr := unsafe.Pointer(ms)
    for i := 0; i < 2; i++ {
        c := (*int)(unsafe.Pointer((uintptr(ptr) + uintptr(8*i))))
        *c += i+1
        fmt.Printf("[%p] %d\n", c,*c)
    }
}
func main() {
    a := &MyStruct{i: 40, j :50}
    MyFunction(a)
    fmt.Printf("[%p]  %v\n", a, a)
}
/*
[0xc00012a010] 41
[0xc00012a018] 52
[0xc00012a010]  &{41 52}
*/
```

&emsp;go函数调用栈布局

```txt


```
