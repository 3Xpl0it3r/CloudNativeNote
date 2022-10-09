
&emsp;存在代码段里面的字符会被标记为SRODARA,在go语言只是不支持直接修改`string`类型的变量的内存空间,但是可以在string和[]byte之间转换来达到实现修改目的
- 先将这段内存数据copy到堆/栈上
- 将变量转换为[]byte类型,修改字节数据
- 将修改后的字节转换为string类型



#### 数据结构
```go
type StringHeader struct {
	Data uintptr
	Len  int
}
```
&emsp;string可以看成是只读的slice;
