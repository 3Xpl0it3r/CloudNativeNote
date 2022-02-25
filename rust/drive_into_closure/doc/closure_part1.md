**常规例子**
&emsp;在main.rs里面让我们创建第一个closure：
```rust
    let add = |x ,y| x + y;
```

**Returnning Closure**
&emsp;我们不仅仅可以接收closure，我们同样可以返回一个函数。
```rust

fn main(){
    let closure = returns_closures();
    println!("Closure(1) => {}", closure(1));
    receives_closure_one_parment(returns_closures());
}

fn returns_closures() -> impl Fn(i32) -> i32 {
    |x| x + 4
}
```
&emsp; 我们可以直接使用，或者返回一个同样的闭包类型，然后把他当作参数传给其他函数。




**Rice and Curry**
&emsp;有了上面的例子，我们可以尝试写一个`curry`的闭包。我们想要写一个函数，这个函数接受一个闭包(这个闭包里面接收两个参数)。形象的表示就是如下： 给定一个函数`f(x,y)`, 我们想要`curry(f, a)`,然后返回一个`g(y)`的函数。 也就是`g(y) => f(a, y)`;
&emsp;首先我们的函数签名如下:
```rust 
fn curry<F>(f: F, x : i32) -> impl Fn(i32) -> i32 
where
    F:Fn(i32, i32) -> i32,
{
    |y| f(x, y)
}

```
