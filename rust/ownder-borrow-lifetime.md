&emsp;在rust里面，当我们传递一个资源给其他对象的时候，那么这个就意味着有`ownership`的传递-谁拥有资源的人，就可以对这个资源做他想做的事情(owner控制资源创建/释放， mut控制资源读写)， owner 就负责对资源的清理操作， 看下下面的例子:
```rust
fn print_sum(v :Vec<i32>) {
    println!("{}", v[0] + v[1]);
}

fn main(){
    let mut v = Vec::new();
    for i in 1..1000 {
        v.push(i);
    }

    print_sum(v);
    println("All don");
}
```
&emsp;`ownership`转移的过程在rust里面也被称为是`move`. 因为资源已经从老的地方被移动到了新的地方(这个在底层其实也是地址的转移，当然rust有自己权限控制， 这个过程看起来新建一个本地变量，将老的变量里面的地址拷贝到新的变量里面，然后删除俩的变量， 伪代码看起来如下, 但是未必正确，只是个人理解, 这个底层的原理涉及到语言设计层面,有一套类型系统来保证，我也不懂), 这个过程几乎没有任何性能的损失(这个操作都是在栈上)， 只是弱引用被移动了而已。
```rust
    let a = 0x0000;
    let b = 0x0000
    drop(a);
```

&emsp;在rust里面`Move`和`Copy`是不一样的概念，(这个其实在底层他们的行为基本类似，对于基础类型拷贝value，所以对于基础类型几乎不存在什么所有权转移，对于复杂类型byte-by-byte的copy指针)。可以通过下面的例子来证明下：
```rust
     Running `target/debug/example`
a_ptr: 0x109181579  b_prt: 0x109181579
0x7fdda9c05c40
0x7fdda9c05c40
➜  example git:(master) ✗ cat src/main.rs


fn example1(){
    let a: &str = "dsdsa";
    let b = a;
    println!("a_ptr: {:p}  b_prt: {:p}", a.as_ptr(), b.as_ptr());
}

fn example2(){
    let a: String = String::from("hello world");
    println!("{:p}", a.as_ptr());
    let b: String = a;      // 在move之后a 被认为就不存在了 
    println!("{:p}", b.as_ptr()); // 此时a里面的变量就被删除了,相当于未初始化变量， 在rust 里面 let a :String ;这种是不允许的
    // let a :String = String::from("hello"); //如果想要下面代码可用，必须重新给a初始化
    // println!("{}", a); // 编译报错， borrow moved value

}


fn main(){
    example1();
    example2()
}
```
&emsp;上面的例子example2 在移动之后a就被认为不存在了(个人理解：rust里面的move相当于把a里面的值copy到b里面，然后把a里面的地址删除掉，删除类型未初始化，rust里面不允许使用未初始化的值，这个是不合法的，如果想要用a, `let a = String::from("saasas")`, 这个时候a又会变得可用)

&emsp; 在rust里面几乎任何一个涉及到赋值操作的地方都会有移动，当然rust里面move操作在赋值/函数传惨/函数返回值都会有所有权移动(这个其实也不理解，类型其他语言函数传参都会提到传参值传参，在函数里面新建一个临时变量，将参数赋值给临时变量--所以函数传参和返回值都会有所有权移动的问题)

&emsp;考虑rust官方的一个例子`pub fn with_capacity(capacity: usize) -> String`, 这个例子是给string分配一段内存空间，然后把他返回给调用者。这个时候所有权就转移了，函数也不用关心这个buffer最终属于谁，这个是调用者关心的事情，调用者对这个string/buffer拥有了所有权，也有责任负责清理掉他们。(这个其实和其他语言里面没啥区别，例如函数`strdup`它会分配一段内存空间，让用户来处理，当然也期望用户来管理和最终释放它，当然有点不同的时候在类型c语言里面会造成二次释放的问题。)

**Borrowing**
&emsp;使用borrowing的几种原因：
- 允许多个指针指向同一个资源， 共享(但是不能删除)
- reference 和c里面指针非常像(我一般就当成指针来看看)
- 一个引用也是一个对象。可变的引用被move，不可变的引用被copy(当一个引用被drop掉了，那么借用就到此结束了)
- 简单的理解下，引用只是就像来回转移所有权，只不过没有那么明显
```rust
// 可以通过这种来回转移所有权的办法来做到和reference同样的效果
fn print_sum_take_and_back_ownership(v: Vec<i32>) -> Vec<i32> {
    println!("{}", v[0]+v[1]);
    v
}

fn print_sum_take_reference_v1(v: &Vec<i32>) {
    println!("{}", v[0] +  v[1]);
}

fn print_sum_take_reference_v2(v: &Vec<i32>) {
    // let tmp_v: &Vec<i32> = v;    // 
    // 可以通过解引用来获取变量值，可以确信引用就是指针
    println!("{}", (*v)[0] +  (*v)[1]);
    
    // v/tmp_v 在函数结束后，本身会被drop掉，但是所有权还是和第一个例子一样的效果一样
}

fn main(){
    let mut v = Vec::new();
    for i in 1..1000 {
        v.push(i);
    }

    v = print_sum_take_and_back_ownership(v);

    print_sum_take_reference_v1(&v);
    print_sum_take_reference_v2(&v);
}
```
&emsp;在上面的例子，无论上面三种那种，main都会对vector拥有所有权，但是和第一种对比他们还是有点差异的，在后面两种方式里面，main 对vector的控制会有点点的限制，main不能对vector做修改(但其实也可以修改，不过要代码顺序上做些调整了)， 第三个例子能够正常的work起来，是因为rust的自解引用的规则。
```rust
// takes v by (immutable) reference
fn count_occureences(v: &Vec<i32>, val: i32) -> usize {
    v.into_iter().filter(|&&x| x == val).count()
}

fn main(){
    let v = vec![2,9,3,1,3,2,5,2];

    // borrowing v for the iterator
    for &item in &v {
        // the first borrow is still active
        // we borrow it the second time here!
        let res = count_occureences(&v, item);
        println!("{} is repreated {} times", item, res);
    }
}
```
&emsp;所有这些申请引用和解除引用都是在编译时完成的。看下一个例子
```rust 
fn middle_name(full_name: &str) -> &str {
    full_name.split_whitespace().nth(1).unwrap()
}


fn main(){
    let name = String::from("Harry James Potter");
    let res = middle_name(&name);
    assert_eq!(res, "James");
}
// 错误的代码
fn main(){
    let res;
    {
        let name = String::from("Harry James Potter");
        res = middle_name(&name);
        // res指向name上面一段内存，但是离开作用域后name就被释放了
    }
    assert_eq!(res, "James");
}
```

**Lifetimes**
&emsp;在rust里面所有的资源都会生命周期。他们被创造出来，然后到最终消亡。`lifetime`在某种程度上面可以看成作用域，代码块, 但是并不完全正确，因为变量是可以在代码快之间移动的. rust里面是不允许我们在指向一个未创建或者已经被释放掉的对象的引用。除此之外和owndership没啥不一样（lifetime主要就是针对引用的);这个应该也是rust里面最难的地方了。

