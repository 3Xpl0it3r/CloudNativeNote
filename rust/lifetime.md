<link https://eta.st/2016/04/16/lyar-lifetimes.html>
&emsp;rust 的lifetime主要解决两件事情：
- **内存管理**: rust 可以自动的管理内存，也不需要通过gc或者引用计数器来管理内存，编译器能够自动识别对象的生命周期，以及在正确的地方插入内存释放的代码， rust 同样解决了内存二次释放的问题。
- **竞争条件**: rust有着严格的owner规则，当有多个线程的时候，有且只有一个线程对这个对象有所有权，这个避免在多线程编程里面很多错误，大部分情况下我们就可以做到无锁编程。

&emsp;当我们在研究borrow的时候，下面有2条规则限制了我们可以做哪些事情，哪些是不可以做的。
- 任何一个借用的生命周期都不能比owner的要长(这个很容易理解，如果可以那么就有悬浮指针的问题)
- 在同一个时刻，你要么只能有一个可变借用，或者1+个不可变借用（二选一）
> 上面的规则rust编译器就是通过`lifetime`来检测我们是否遵守了规则

&emsp;`lifetime`主要用来描述一个object可以存活多久。当你创建了一个的变量，编译器就会给他附加一个生命周期。然后编译器会将这个变量的生命周期和其他的生命周期做比较，从而检测你的代码是不是正确。下面一段话来自`Rustnomicon`(这是一本将所有权的 onlinebook)
`Rust enforces [the borrowing rules] through lifetimes. Lifetimes are effectively just names for scopes somewhere in the program. Each reference, and anything that contains a reference, is tagged with a lifetime specifying the scope it's valid for.`
> lifetime可以看成变量在代码里面的作用域

&emsp;记住当你的代码离开作用域的时候，变量也会离开。变量离开的顺序和它创建的顺序相反：
```rust
// 
struct RefObject<'x>(&'x u32);

fn steal_a_var<'x>(o: RefObject<'x>) {
    println!("{}", o.0)
}

fn main(){
    // a is created in main()'s scope
    let a = 3;

    // b is created in main()'s scope
    let mut b = &a;

    // c is created in main()'s scope
    let c = RefObject(&b);

    // c is moved to steal_a_var;
    steal_a_var(c); // c'owner is trans into steal_a_var functions
    // c is now invalied

    // d is created in main()'s scope
    let d = &mut b ;
}
// main()'s scope ends, killing all the vaiables, in sidoe
// d goes away
// b goes away
// a goes away
```

&emsp;但是实际情况发生会比上面的例子更为微妙。想下下面的例子:当一个变量被创建的时候他就有了一个属于自己的小型作用域，直到这个作用域一直包含到他生命周期结束。d先离开，这是因为d的作用域要在b之前结束。重写下上面的例子，使得作用域更为清晰点：
```rust
fn main(){
    'a:{
        // a is created, it gets it owns scope,// 它会一直持续到这个作用域包含它
        let a = 3;

        'b: {
            // b 被创建，it gets is owns scope, ,lasting as long as the scope contains it
            let mut b = &a;

            'c: {
                // c is created , it get its owns scope, 'c
                // only lasting until steal_a_var(), as it is removed
                let c = RefObject(&b);
                steal_a_var(c);
                {
                    // c 
                }
            } // c goes away
            'd: {
                let d = &mut b;
            } //d goes away
        } // b goes away
    } // a goes away
}
```
&emsp;rust 会检查所有的生命周期：
- 它看到`'a`要比`'c`和`'d`都要长，并且知道没有借用比它生存的长点。
- 它看到`'c`和`'d` 是彼此分离的。并且知道可变借用规则不会被打破。
&emsp;这个看起来很复杂，但是rust会自动帮我们创建这些作用域和管理他们，这个在大部分场景下都是适用的。然而rust并不能完全的覆盖所有的case。有时候它需要得到更多的提示，我们需要把这些scope信息写到代码里面，以便rust能理解。

example1:错误例子
**在structure里面存储borrow**
```rust
struct Object {
    number: u32,
}
struct Multiplier{
    object: &Object,
    mult: u32,
}

fn print_borrower_number(mu: Multiplier) {
    println!("Result: {}", mu.object.number * mu.mult)
}

fn main(){
    let obj = Object{number: 5};
    let obj_times_3 = Multiplier{object: &obj, mult: 3}
    print_borrower_number(obj_times_3);
}
```
报错信息如下:
```bash
    Checking example v0.1.0 (/Users/l0calh0st/Git/l0calh0st.cn/CloudNativeNote/rust/example)
error[E0106]: missing lifetime specifier
 --> src/main.rs:6:13
  |
6 |     object: &Object,
  |             ^ expected named lifetime parameter
  |
help: consider introducing a named lifetime parameter
  |
5 ~ struct Multiplier<'a> {
6 ~     object: &'a Object,
  |

For more information about this error, try `rustc --explain E0106`.
error: could not compile `example` due to previous error
```
上面报错是rust没有办法为我们识别lifetime，现在让我们分析下里面逻辑：
- 我们声明了一个`Object`变量，里面有一个单个的number，rust很容易知道`number`是和`Object`存活一样的长。
- 下面我们声明了`Multiplier`的变量，里面包含一个`mult`的u32变量，和一个指向`Object`的引用，rust对于mult很开心，因为它知道mult将会和Object生存的一样长。但是它不喜欢`object`这个玩意
- 现在问题是object是有可能存储任意时间的，有可能要比`Multiplier`的生存时间短，rust希望我们约束下`Multiplier`。
```rust
// 解决方案
struct Multiplier<'a>{
    object: &'a Object,
    mult: u32,
}
```
> 没错，语法很复杂。
&emsp;上面代码解释:我有一个`Multiplier`类型的变量，它的生命周期是'a,在这个生命周期里面它会和'a生存一样长。在给定生命周期的情况下，我在`Multiplier`这个对象里面有一个object的对象，它的生存时间至少要和'a一样长。 rust在内部就会将`Multiplier`的生命周期和 `Object`的生命周期关联起来，以确保`Object`不会被清理，在`Multiplier`离开之前。

**问题2: Borrowed typed  in functons**
```rust 
struct Object{
    number: u32,
}

struct Multiplier {
    mult: u32,
}

fn object_combinator(a: &mut Object, b: &Object) -> &mut Object{
    a.number = a.number + b.number;
    a
}

fn main(){
    let mut a = Object{number:3};
    let b = Object{number:4};
    println!("Result: {}", object_combinator(&mut a, &b));
}

```
会有如下报错信息：
```bash
error[E0106]: missing lifetime specifier
  --> src/main.rs:10:53
   |
10 | fn object_combinator(a: &mut Object, b: &Object) -> &mut Object{
   |                         -----------     -------     ^ expected named lifetime parameter
   |
   = help: this function's return type contains a borrowed value, but the signature does not say whether it is borrowed from `a` or `b`
help: consider introducing a named lifetime parameter
```
&emsp;这个问题和上面描述的差不多，给函数`object_combinator`传递了两个引用，这两个引用有可能有不同的生命周期，然后返回以借用。rust要如何知道你返回的借用能够生存多久？ 他不知道，所以rust希望我们帮他弄清楚。

&emsp;我们知道我们的函数`object_combinator` 将第二个`Object`的对象b里面的number增加到a上面，然后返回a.， 因此我们需要关心a的寿命，因为这个是个返回的。 解决方案如下：
```rust
struct Object{
    number: u32,
}

struct Multiplier{
    mult: u32,
}

fn object_combinator<'a, 'b>(a: &'a mut Object, b: &'b Object) -> &'a mut Object{
    a.number = a.number + b.number;
    a
}

fn main(){
    let mut a = Object{number:3};
    let b = Object{number:4};
    let a = object_combinator(&mut a, &b);
}
```
&emsp;上面代码相当于告诉rust，我有一个`object_combinator`函数，接收两个参数，一个是生产周期和`'a`一样长的`Object` 对象，另外是生命周期和`'b`一样唱的`Object`对象.，想要借用一个对象，这个对象的生命周期要和`'a`一样长， rust这样就清楚了。


**‘Static**
&emsp;`'static` 这个生命周期是一个非常特殊的生命周期，意味着它的生命周期是整个程序的生命周期。你也许会看到下面的代码
```rust
let string: &'static str = "I am string";
```
毫无疑问自然还有static的事情。
```rust 
static UNVERISE_ANSWER:u32 = 42;
let answer_borrow: &'static = &UNVERISE_ANSWER;
```
当在尝试多线程编程的时候，也会经常看到这样的事情。给另外一个线程传递一个生命周期不是'static的引用在rust里是不被允许的。因为接收对象的线程有可能要比发送端的那个线程要生存的长。


**Lifetime Elision**
&emsp;`lifetime elision`是rust会自动帮我们在底层添加上这些生命周期的东西，以便程序不那么难以阅读

&emsp;基本上有如下规则
- 生命周期进入函数，被称为输入生命周期
- lifetime在return语句上，被称为输出生命周期
- 如果只有一个输入生命周期，那么这个生命周期会被应到到所有的输出生命周期上
```rust
fn foo<'a>(bar: &'a str) -> &'a str {}
```
- 如果有多个输入生命周期，并且至少有一个引用是指向自己的，那么这个self的生命周期会被应用到所有的输出生命周期上面

