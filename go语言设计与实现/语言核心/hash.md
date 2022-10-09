#### 设计原理
&emsp;hash函数的选择决定hash表的读写性能: hash函数的两种方式:
- 开放寻址方法
> 依次探测和比较数组里面的元素，用来判断目标键值是否在于hash表里面 ----- 这种方式实现hash表的底层数据结构就是数组
> `index := hash("author") % arrary.len()`
> 影响它的因素装载因子(数组中元素与数组大小的比值),最坏情况下检索时间复杂度为O(arrary.len())
- 拉链法
> 实现方式数组+连表,有些hash引入红黑树来提高hash性能
> 存储各个节点的内存都是动态申请的,比较节省存储空间,一般使用连表数组来表示底层数据
> `index := hash("key") % array.len()` 找到对应的桶，然后遍历连表(这个可以压缩到o(longn))
> 最好的情况下，每个桶里面只有1-2个元素

#### 底层数据结构
```go
type hmap struct {
	count     int // # 当前hash表中元素的个数
	flags     uint8
	B         uint8  // 桶的数量
	noverflow uint16 // approximate number of overflow buckets; see incrnoverflow for details
	hash0     uint32 // hash种子，这个是创建hash的时候就确定了，每个hash的hash随机种子是不固定的

	buckets    unsafe.Pointer // 
	oldbuckets unsafe.Pointer // 用于保存hash 在扩容之前保存的bucket的字段,大小是当前bucket的一半
	nevacuate  uintptr        // progress counter for evacuation (buckets less than this have been evacuated)

	extra *mapextra // optional fields
}
// mapextra 额外字段 bmap是hmap的桶, 
type mapextra struct {
	overflow    *[]*bmap
	oldoverflow *[]*bmap

	// nextOverflow holds a pointer to a free overflow bucket.
	nextOverflow *bmap
}
type bmap struct { // hash桶
// 每个桶只有8个元素,  当hash元素过多，单个桶超过8个元素时，就会使用nextOverflow这个元素
	tophash [8]uint8    // 记录keyhash的高8位，通过比较key的高8位可以减少访问建值对提高性能
    // runtime 期间添加字段
    keys [8]keytype
    values [8]valuetype
    pad uintptr
    overflow uintptr
}
// 在runtime阶段不止一个tophash字段
// 在1.18之前不支持范性,所以keyvalue大小只能在编译器期间推到
```


#### 初始化
&emsp;hashmap初始化有两种方式: 字面量和运行时
##### 字面量 
```go
// 初始化map通过字面量这种方式，会通过maplit这种方式来初始化
hash := map[string]int{
    "1": 1,
    "2": 2,
    "3": 3,
}
func maplit(n *Node, m *Node, init *Nodes) {
	// make the map var
	if len(entries) > 25 { // 元素个数大雨25个情况
		// loop adding structure elements to map
        hash := make(map[string]int, 26)
        vstatk := []string{"1", "2", "3", ... ， "26"}
        vstatv := []int{1, 2, 3, ... , 26}
        for i := 0; i < len(vstak); i++ {
            hash[vstatk[i]] = vstatv[i]
        }
        return
	}
    // 元素个数小与25个时
    // 直接通过var[c] = expr 这种方式添加
	// For a small number of entries, just add them directly.
    hash := make(map[string]int, 3)
    hash["1"] = 1
    hash["2"] = 2
    hash["3"] = 3
}
```
&emsp;字面量初始化方式都是通过make关键字来创建
##### 运行时
&emsp;当创建的hash被分配到栈上,并且BUCKETSIZE = 8时,go编译器使用下面方式快速初始化:
```go
var h *hmap
var hv hmap
var bv hmap
h := &hv
b := &bv
h.buckets = b
h.hash0 = fastrand0()
```
&emsp;使用map创建hash,go 会在类型检查阶段将他们转化为`runtime.makemap`(字面量初始化也是go提供的语法糖,背后也是调用makemap)
```go
func makemap(t *maptype, hint int, h *hmap) *hmap {
	mem, overflow := math.MulUintptr(uintptr(hint), t.bucket.size) //计算所需要内存大小
	// initialize Hmap
	if h == nil {
		h = new(hmap)
	}
	h.hash0 = fastrand() // 获取随机种子

	// Find the size parameter B which will hold the requested # of elements.
	// For hint < 0 overLoadFactor returns false since hint < bucketCnt.
	h.B = B

	// allocate initial hash table
	// if B == 0, the buckets field is allocated lazily later (in mapassign)
	// If hint is large zeroing this memory could take a while.
	if h.B != 0 {
		var nextOverflow *bmap
		h.buckets, nextOverflow = makeBucketArray(t, h.B, nil) //创建用于保存桶的数组
	}
	return h
}
// makeBucketArray 创建用于保存桶的数组
func makeBucketArray(t *maptype, b uint8, dirtyalloc unsafe.Pointer) (buckets unsafe.Pointer, nextOverflow *bmap) {
	base := bucketShift(b)
	nbuckets := base
	// For small b, overflow buckets are unlikely.
	// Avoid the overhead of the calculation.
	if b >= 4 {
        // 当桶的个数大于2^4 个数时, 会额外创建2^(b-4)个桶
	}
	buckets = newarray(t.bucket, int(nbuckets))
	return buckets, nextOverflow
}
```

#### 读写操作
&emsp;hash的访问一般一般通过下标或者遍历方式来访问:
```go
hash[key]=value
// delete
delete(hash, key)
```

##### 访问
&emsp;上面类似的操作在编译类型检查阶段会被转换为`OINDEXMAP`的操作,并且在SSA 阶段 会把这两个操作转换如下代码(`cmd/compile/internal/gc.walkexpr`):
```go
v   := hash[key]  // -------> v := *mapaccess1(maptype, hash, &key)
v,  ok := hash[key]     // -------> v := *mapaccess2(maptype, hash, &key)
```
&emsp; `mapaccess1`方法实现
```go
func mapaccess1(t *maptype, h *hmap, key unsafe.Pointer) unsafe.Pointer {
	hash := t.hasher(key, uintptr(h.hash0))     // 通过hasher函数来计算当前key的hash
	m := bucketMask(h.B)                        // 获取桶的mask值
	b := (*bmap)(add(h.buckets, (hash&m)*uintptr(t.bucketsize))) // 找到对应的桶
bucketloop:
	for ; b != nil; b = b.overflow(t) {                 // 遍历正常桶和溢出桶里面hash(高8位)
        // 先比较hash
        // 在比较value
    }
}
```
&emsp;`mapaccess2`方法实现
```go
func mapaccess2(t *maptype, h *hmap, key unsafe.Pointer) (unsafe.Pointer, bool) {
    // 和mapaccess1的逻辑基本类似, 不过找到返回true，找不到返回false
}
```


##### 赋值
&emsp;当`hash[key]`在左侧时候,在SSA阶段会被编译成`runtime.mapassign`
```go
// Like mapaccess, but allocates a slot for the key if it is not present in the map.
func mapassign(t *maptype, h *hmap, key unsafe.Pointer) unsafe.Pointer {
	hash := t.hasher(key, uintptr(h.hash0))     // 获取hash

	if h.buckets == nil {
		h.buckets = newobject(t.bucket) // newarray(t.bucket, 1) // 如果桶为空,创建一个新的桶
	}
again:
	bucket := hash & bucketMask(h.B)            // 获取桶
	b := (*bmap)(unsafe.Pointer(uintptr(h.buckets) + bucket*uintptr(t.bucketsize))) // 获取hash 不对应的桶
	top := tophash(hash)                                                            // 获取hash高8位

bucketloop:
	for {           
		for i := uintptr(0); i < bucketCnt; i++ {
            // for 循环遍历正常桶和溢出桶存储数据,通过判断tophash,key是否相等/ 遍历完跳出
		}
	}
	return elem
}
```

##### 扩容
&emsp;hash元素越多性能会逐渐恶化,因此需要跟多的桶和更大的内存才能保证性能,`runtime.mapassign`会在如下两种情况下触发扩容:
- 装载因子超过6.5
- hash使用了太多溢出桶

