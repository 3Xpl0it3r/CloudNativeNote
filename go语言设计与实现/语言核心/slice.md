**切片**
&emsp;由于slice是动态的,因此声明只需要指定切片元素类型
```go
[]int
[]interface{}
```
&emsp;切片在编译期间生成的类型只会包含切片中元素的类型,即int 或者interface{}等

#### 创建
```go
func NewSlice(elem *Type) *Type {
	if t := elem.Cache.slice; t != nil {
		if t.Elem() != elem {
			Fatalf("elem mismatch")
		}
		return t
	}

	t := New(TSLICE)
	t.Extra = Slice{Elem: elem}
	elem.Cache.slice = t
	return t
}
```
&emsp;在这个里面Extra只包含一个类型的资源(但是在arrary里面却有一个bound的字段),类型是在编译期间就确定好了的.


#### 数据结构
&emsp;编译期间切片是`types.Slice`确定的,运行时可以由`reflect.SliceHeader`结构表示
```go 
type SliceHeader struct {
	Data uintptr            // 指向数组的指针
	Len  int                // 切片长度
	Cap  int                // 当前切片容量,即Data数组的大小
}
```
&emsp;切片是在数组之上做了一层抽象,提供了对数组连续片段的引用(提供了一个访问数组的指针)

#### 初始化
&emsp;go里面提供了三种初始化的方式
- 下标方式获得初始化切片方式
- 使用字面量初始化切片
- 使用关键字`make`创建切片

#### 使用下标
&emsp;这是最接近汇编的方式,也是所有方式里面最底层的一种.编译器会将arr[:3]或者slice[:3]等语句转换成`OpSliceMake`等操作,
```go 
package opslicemake
func newSlice()[]int{
    arr := [3]int{1,2,3}
    slice := arr[0:1]
    return slice
}
```
> 转化后的代码如下:
```
v11 (5) = MOVQload <*[3]int> [8] v2 v9 (&arr[*[3]int], slice.ptr[*int])
v14 (?) = MOVQconst <int> [1] (slice.len[int])
name &arr[*[3]int]: v11
name slice.ptr[*int]: v11
name slice.len[int]: v14
name slice.cap[int]: v17
```

#### 字面量
&emsp;当使用字面量`[]int{1,2,3}`创建切片时,`cmd/compile/internal/gc.slicelit`会在编译期间将它展开
```go
var vstat [3]int
vstat[0] = 1
vstat[1] = 2
vstat[2] = 3
var vauto *[3]int = new([3]int)
*vauto = vstat
slice := vauto[:]
```
> 先创建一个数组,然后针对数组提取slice转化为第一种方式

#### 关键字
&emsp;`make`关键字来创建
```go
func typecheck1(n *Node, top int) (res *Node) {
	switch n.Op {
	case OMAKE:
		switch t.Etype {
		case TSLICE:
			if !checkmake(t, "len", l) || r != nil && !checkmake(t, "cap", r) {     // 检查len/cap参数 len参数是必须的
				return n
			}
			if Isconst(l, CTINT) && r != nil && Isconst(r, CTINT) && l.Val().U.(*Mpint).Cmp(r.Val().U.(*Mpint)) > 0 { // 检查参数
                // 确保len < cap
				return n
			}
			n.Left = l
			n.Right = r
			n.Op = OMAKESLICE// 转化为OMAKESLICE  节点
		}
    }
}
// walkexpr会跟将OMAKESLICE 拆分两个不通的分支(判断是否需要逃逸)

func walkexpr(n *Node, init *Nodes) *Node {
	case OMAKESLICE:
		l := n.Left
		r := n.Right
		t := n.Type
		if t.Elem().NotInHeap() {   // 逃逸分析
			yyerror("%v can't be allocated in Go; it is incomplete (or unallocatable)", t.Elem())
		}
		if n.Esc == EscNone { //不会发生逃逸
			// var arr [r]T
			// n = arr[:l]
		} else {  // 发生了逃逸
			// n escapes; set up a call to makeslice.
			len, cap := l, r
			fnname := "makeslice64"
			if (len.Type.IsKind(TIDEAL) || maxintval[len.Type.Etype].Cmp(maxintval[TUINT]) <= 0) &&
				(cap.Type.IsKind(TIDEAL) || maxintval[cap.Type.Etype].Cmp(maxintval[TUINT]) <= 0) {
				fnname = "makeslice"
				argtype = types.Types[TINT]
			}
            // 调用makeslice / makeslice4 来创建slice 
		}
}
// 逃逸情况下makeslice 创建方式 上面的都是编译时的消耗 runtime.slice.go
//  返回一个新的指针  和原来的不在相关
func makeslice(et *_type, len, cap int) unsafe.Pointer {
    // mem = et.size * cap
	mem, overflow := math.MulUintptr(et.size, uintptr(cap))
	return mallocgc(mem, et, true)
}
```


#### 访问元素
&emsp;`len`和`cap`是获取slice的长度和cap,但是在编译器看来这是两个操作`OLEN`和`OCAP`,`cmd/compile/internal/gc.state.expr`函数会在SSA阶段将他们分别转换成`OpSliceLen`和`OpSliceCap`:
```go
func (s *state) expr(n *Node) *ssa.Value {
	case OLEN, OCAP:
		switch {
		case n.Left.Type.IsSlice():
			op := ssa.OpSliceLen
			if n.Op == OCAP {
				op = ssa.OpSliceCap
			}
			return s.newValue1(op, types.Types[TINT], s.expr(n.Left))
}
```
&emsp;在一些情况下会直接替换成在某些情况下会直接替换成切片长度或者容量(直接去地址，并没有运行时的开销)
```
b2: ← b1-
v28 (5) = Const64 <int> [3]
v29 (5) = SliceMake <[]int> v10 v28 v28 (a[[]int])
v30 (+6) = Const64 <int> [3] (length[int])
v31 (+7) = Const64 <int> [3] (capacity[int])
v33 (8) = VarDef <mem> {~r0} v22
v34 (+8) = Store <mem> {int} v4 v30 v33
v35 (8) = VarDef <mem> {~r1} v34
v36 (8) = Store <mem> {int} v5 v31 v35
```
&emsp;Index操作也会在中间代码阶段转换为对内存地址的直接访问, 切片的操作基本都是在编译期间完成的,编译期间会将range转化为for普通循环;


#### 追加和扩容
&emsp;`append`向slice里面追加元素,中间代码生成阶段,
```go
func (s *state) append(n *Node, inplace bool) *ssa.Value {
	// If inplace is false, process as expression "append(s, e1, e2, e3)":
    // 不会影响原来的切片, makeslice 会返回一个新地址类似malloc()
	ptr, len, cap := s
	*(ptr+len) = e1
	*(ptr+len+1) = e2
	*(ptr+len+2) = e3
	return makeslice(ptr, newlen, cap)
	
	
	// If inplace is true, process as statement "s = append(s, e1, e2, e3)":
    //  会覆盖原来的切片地址
	a := &s
	ptr, len, cap := s
	newlen := len + 3
	if uint(newlen) > uint(cap) {
	   newptr, len, newcap = growslice(ptr, len, cap, newlen)
	   *a.cap = newcap // write before ptr to avoid a spill
	   *a.ptr = newptr // with write barrier
	}
	*a.len += *a.len + 3
	*(ptr+len) = e1
	*(ptr+len+1) = e2
	*(ptr+len+2) = e3
}
// growslice
func growslice(et *_type, old slice, cap int) slice { // cap > old.cap
    // cap基于某些策略, growslice的代价是昂贵的, mallocgc申请内存-> 将原始的arrary给copy
    var p ptr = mallocgc(cap)
	memmove(p, old.array, lenmem)
	return slice{p, old.len, newcap}
}

// growslice
func growslice(et *_type, old slice, cap int) slice {
	if et.size == 0 { // 
		// append should not create a slice with nil pointer but non-zero len.
		// We assume that append doesn't need to preserve old.array in this case.
		return slice{unsafe.Pointer(&zerobase), old.len, cap}
	}

	newcap := old.cap
	doublecap := newcap + newcap
	if cap > doublecap { //如果cap是当前cap的2倍，则newcap = cap
		newcap = cap
	} else { 
		if old.len < 1024 { // 如果old.len < 1024 元素
			newcap = doublecap  // newcap为当前cap的2倍
		} else {
			// Check 0 < newcap to detect overflow
			// and prevent an infinite loop.
			for 0 < newcap && newcap < cap {    // 如果当前cap2倍 < cap， 那么 newcap 扩容1/4
				newcap += newcap / 4
			}
			// Set newcap to the requested cap when
			// the newcap calculation overflowed.
			if newcap <= 0 { // 如果newcap <0  ,那么 newcap = cap, 防止溢出
				newcap = cap
			}
		}
	}

	var overflow bool
	var lenmem, newlenmem, capmem uintptr
	// Specialize for common values of et.size.
	// For 1 we don't need any division/multiplication.
	// For sys.PtrSize, compiler will optimize division/multiplication into a shift by a constant.
	// For powers of 2, use a variable shift.
	switch {
	case et.size == 1:
		lenmem = uintptr(old.len)
		newlenmem = uintptr(cap)
		capmem = roundupsize(uintptr(newcap))
		overflow = uintptr(newcap) > maxAlloc
		newcap = int(capmem)
	case et.size == sys.PtrSize:
		lenmem = uintptr(old.len) * sys.PtrSize
		newlenmem = uintptr(cap) * sys.PtrSize
		capmem = roundupsize(uintptr(newcap) * sys.PtrSize)
		overflow = uintptr(newcap) > maxAlloc/sys.PtrSize
		newcap = int(capmem / sys.PtrSize)
	case isPowerOfTwo(et.size):
		var shift uintptr
		if sys.PtrSize == 8 {
			// Mask shift for better code generation.
			shift = uintptr(sys.Ctz64(uint64(et.size))) & 63
		} else {
			shift = uintptr(sys.Ctz32(uint32(et.size))) & 31
		}
		lenmem = uintptr(old.len) << shift
		newlenmem = uintptr(cap) << shift
		capmem = roundupsize(uintptr(newcap) << shift)
		overflow = uintptr(newcap) > (maxAlloc >> shift)
		newcap = int(capmem >> shift)
	default:
		lenmem = uintptr(old.len) * et.size
		newlenmem = uintptr(cap) * et.size
		capmem, overflow = math.MulUintptr(et.size, uintptr(newcap))
		capmem = roundupsize(capmem)
		newcap = int(capmem / et.size)
	}

	// The check of overflow in addition to capmem > maxAlloc is needed
	// to prevent an overflow which can be used to trigger a segfault
	// on 32bit architectures with this example program:
	//
	// type T [1<<27 + 1]int64
	//
	// var d T
	// var s []T
	//
	// func main() {
	//   s = append(s, d, d, d, d)
	//   print(len(s), "\n")
	// }
	if overflow || capmem > maxAlloc {
		panic(errorString("growslice: cap out of range"))
	}

	var p unsafe.Pointer
	if et.ptrdata == 0 {
		p = mallocgc(capmem, nil, false)
		// The append() that calls growslice is going to overwrite from old.len to cap (which will be the new length).
		// Only clear the part that will not be overwritten.
		memclrNoHeapPointers(add(p, newlenmem), capmem-newlenmem)
	} else {
		// Note: can't use rawmem (which avoids zeroing of memory), because then GC can scan uninitialized memory.
		p = mallocgc(capmem, et, true)
		if lenmem > 0 && writeBarrier.enabled {
			// Only shade the pointers in old.array since we know the destination slice p
			// only contains nil pointers because it has been cleared during alloc.
			bulkBarrierPreWriteSrcOnly(uintptr(p), uintptr(old.array), lenmem-et.size+et.ptrdata)
		}
	}
	memmove(p, old.array, lenmem)

	return slice{p, old.len, newcap}
}
```

#### slice拷贝
&emsp;
```go 
func copyany(n *Node, init *Nodes, runtimecall bool) *Node {

	if runtimecall { // 在运行时copy
        // runtime里面执行slicecopy
		fn := syslook("slicecopy")
		return mkcall1(fn, n.Type, init, ptrL, lenL, ptrR, lenR, nodintconst(n.Left.Type.Elem().Width))
	}
    //memmove 内存copy / 编译期间执行的copy
	fn := syslook("memmove")
    memmove(a.ptr, b.ptr, n*sizeof(elem(a)))
}
// slicecopy底层调用的也是memmove 负责copy
func slicecopy(toPtr unsafe.Pointer, toLen int, fmPtr unsafe.Pointer, fmLen int, width uintptr) int {
	size := uintptr(n) * width
	if size == 1 { // common case worth about 2x to do here
		// TODO: is this still worth it with new memmove impl?
		*(*byte)(toPtr) = *(*byte)(fmPtr) // known to be a byte pointer
	} else {
		memmove(toPtr, fmPtr, size)
	}
	return n
}
```
