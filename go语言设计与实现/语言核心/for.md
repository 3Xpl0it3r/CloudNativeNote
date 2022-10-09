&emsp;在go里面无论是`for`还是`for-range`都会被go编译器转化为普通的`for`循环

&emsp;for的两种循环方式
- 经典循环
&emsp;经典循环在在编译器看来是一个OFOR的节点，主要有以下四个节点组成
- 初始化循环的Ninit
- 循环继续条件的Left
- 循环体结束时执行的Right
- 循环体NBody
```go
for  Ninit; Left; Right {
        NBody
}
```
#### 0x01 范围循环
&emsp;相比较经典循环go范围循环使用了for和range两个关键字.但是编译器会在编译阶段会将for-range 转换到经典循环(对编译器而言只是将ORANGE转换为OFOR节点)

##### 数组循环
&emsp;对数组而言go语言有三种不同的遍历方式
- 遍历数组/切片清空
- for range a {}遍历数组不关心索引 
- for i := range e {} 遍历数组切片， 只关心索引
- for i,elem := range a {} 遍历数组/切片，关心索引和数据
```go
func walkrange(n *Node) *Node {
	switch t.Etype {
	case TARRAY, TSLICE:
		if arrayClear(n, v1, v2, a) {
            // 如果发现是清楚元素
			return n
		}
        // for range ha { body }  //针对不关心索引和值的情况
        if v1 == nil {
            break
        }

        // // for v1 := range ha { body }  只关心索引情况
        if v2 == nil {
            body = []*Node{nod(OAS, v1, hv1)}
        }
    }
}
```
example
```go
// 原代码
for i := range a {
	a[i] = zero
}

// 优化后
if len(a) != 0 {
	hp = &a[0]
	hn = len(a)*sizeof(elem(a))
	memclrNoHeapPointers(hp, hn)
	i = len(a) - 1
}
```
> 当操作是清理所有的数组的时候，go语言会直接调用`runtime.memclrNoHeapPointers`或者`runtime.memclrHasPointers`来直接清理内存数据,并且执行完成更新索引



##### 哈希表
##### 字符串
##### channel

