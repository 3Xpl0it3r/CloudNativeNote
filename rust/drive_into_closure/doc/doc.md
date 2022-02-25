**Closure:Anonymous Functions that can capture their Environment**
&emsp;rust 的closure是一个匿名函数，用户可以将这个匿名函数保存在变量里面，或者也可以将闭表当作参数传递给其的函数。你可以在一个地方创建一个闭包，然后在另外一个不同的上下文里面来调用他。和普通的函数不一样，closure可以获取当前作用域里面的变量(创建闭包的作用域)。我在演示这些闭包是如何允许我们重复使用代码，和自定义一些行为。

**Createing An Abstraction of Behavior with Closres**
&emsp;我们会用一个例子来演示，在例子里面我们会展示存储一个在以后会执行的closure是很有用的。在这个里过程里面，我们会讨论closure的语法，类型推断，还有trait。

&emsp;假设这样一种情况，我们要制作一款app用来生产一些自定义的任务。后端是用rust来写的，生成任务的算法主要考虑一下几个因素：用户年龄，身体质量指数，运动偏好，最近的锻炼情况，以及他们指定的强度，在这个算法在这个例子里面不会真正用到。重要的事情这个算法计算过程会需要几秒钟的时间，我们只希望在调用这个算法的时候只调用一次，这样就不会让用户来等待了。
&emsp;我们将使用下面的函数来模拟这个假设的算法`simulated_expensive_calculation()`，这个函数在打印`calculating slowly ...`,并且等待几秒钟，然后返回一个用户输入的值。
```rust
use std::thread;
use std::time::Duration;


fn simulated_expensive_calculation(intensity: u32) -> u32 {
    println!("calculating slowly ...");
    thread::sleep(Duration::from_secs(2));
    intensity
}
```

&emsp;下面就是`main`函数，
