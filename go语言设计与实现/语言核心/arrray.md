#### 数组类型创建
&emsp;编译期间通过如下函数生成数组类型
```go
// NewArray returns a new fixed-length array Type.
func NewArray(elem *Type, bound int64) *Type {
	if bound < 0 {
		Fatalf("NewArray: invalid bound %v", bound)
	}
	t := New(TARRAY)
	t.Extra = &Array{Elem: elem, Bound: bound}
	t.SetNotInHeap(elem.NotInHeap()) // 判断array是在堆上还是在栈上
	return t
}
```

#### 数组初始化
&emsp;两种方式
- 指定数组大小
- [...]T声明
&emsp;上面两种声明方式一样，后面一种在编译阶段也会被转换成前面一种(这个就是编译器的自动推导)


##### 推导过程
- 上限推导
> 当使用[10]T 这种方式声明,那么变量的类型会在检查类型阶段就被提取出来，通过NewArray来创建数组
> 当使用[...]T方式声明的时候,通过`typecheckcomplit`对数组大小推导
```go
func typecheckcomplit(n *Node) (res *Node) {
	// Save original node (including n.Right)
	n.Orig = n.copy()

	setlineno(n.Right)

	// Need to handle [...]T arrays specially. // 这个地方就是用来处理 ... 操作符的
	if n.Right.Op == OTARRAY && n.Right.Left != nil && n.Right.Left.Op == ODDD {
        // 获取最右边元素的类型
		n.Right.Right = typecheck(n.Right.Right, ctxType)
		if n.Right.Right.Type == nil {  // 类型为空，直接return
			n.Type = nil
			return n
		}
		elemType := n.Right.Right.Type  // 元素的类型

        // typecheckarraylit 通过for 遍历的方式来回去元素的个数
		length := typecheckarraylit(elemType, -1, n.List.Slice(), "array literal")

		n.Op = OARRAYLIT
        // 最终还是通过NewArray 函数来创建数组
		n.Type = types.NewArray(elemType, length)
		n.Right = nil
		return n
	}

	n.Right = typecheck(n.Right, ctxType)
	t := n.Right.Type
	if t == nil {
		n.Type = nil
		return n
	}
	n.Type = t

	switch t.Etype {
	default:
		yyerror("invalid composite literal type %v", t)
		n.Type = nil

	case TARRAY:
		typecheckarraylit(t.Elem(), t.NumElem(), n.List.Slice(), "array literal")
		n.Op = OARRAYLIT
		n.Right = nil

	}

	return n
}
> 因此`[]int{1,2,3} 和 [3]int{1,2,3} 是等价的，他们没有任何运行时的开销，只有编译时开销`

```
- 语句转换
> 对于字面量组成的数组，根据元素的数量不同，编译器会在初始化字面量的`gc.anylit`函数里面做两种优化
1. 当元素个数小于等于四个，数组元素直接放在stack上
2. 当元素个数大于四个时候，会将数组的元素放到静态区域，并且在运行时取出(少用字面量初始化数组，会影响内存大小)
```go

func anylit(n *Node, var_ *Node, init *Nodes) {
	t := n.Type
	switch n.Op {
	case OSTRUCTLIT, OARRAYLIT:
		if var_.isSimpleName() && n.List.Len() > 4 {
			// lay out static data   // 当元素个数大于4个时候，会把数据布局到static ata段
			vstat := staticname(t)
			vstat.MarkReadonly()

			ctxt := inInitFunction
			if n.Op == OARRAYLIT {
				ctxt = inNonInitFunction
			}
			fixedlit(ctxt, initKindStatic, n, vstat, init)

			// copy static to var
			a := nod(OAS, var_, vstat)

			a = typecheck(a, ctxStmt)
			a = walkexpr(a, init)
			init.Append(a)

			// add expressions to automatic
			fixedlit(inInitFunction, initKindDynamic, n, var_, init)
			break
		}

		var components int64
		if n.Op == OARRAYLIT {
			components = t.NumElem()
		} else {
			components = int64(t.NumFields())
		}
		// initialization of an array or struct with unspecified components (missing fields or arrays)
		if var_.isSimpleName() || int64(n.List.Len()) < components {
			a := nod(OAS, var_, nil)
			a = typecheck(a, ctxStmt)
			a = walkexpr(a, init)
			init.Append(a)
		}

        // fixedlit 会在编译之前转换成更为普通的语句
		fixedlit(inInitFunction, initKindLocalCode, n, var_, init)
	}
}
```



#### 赋值和访问

##### 访问
&emsp;越界检查
- 静态检查
> 索引值是常量，在编译阶段就会检查
- 动态检查
> 索引是变量，在编译期间无法检查，在运行时检查，在访问之前会编译器会插入一条`IsInBounds <bool> v21 v11语句`


