
**lifetime 背后的原因　**
&emsp;为了理解lifetime，我们首先需要理解他们背后的动机，这个就需要理解borrow 规则：
> 在任意一个时刻，只能有一个可变借用 或者 多个不可变借用 （二者只能二选一) 有了可变就不允许有不可变（从内存角度理解rust）

&emsp;看下面一个例子
```rust 
struct Coords {
    pub x: i64,
    pub y: i64,
}

fn shift_x_twice(coords: &mut Coords, delta: &i64) {
    coords.x += *delta;
    coords.x += *delta;
}

fn main(){
    let mut a = Coords{x: 10, y: 10};
    let delta_a = 10;
    shift_x_twice(&mut a, &delta_a);
    let mut b  = Coords{x: 10, y : 10};
    let delta_b = &b.x;
    // shift_x_twice(&mut b, delta_b); // 错误代码：cannot borrow `b` as mutable because it is also borrowed as immutable

}
```
&emsp;上面代码的问题在于`delta_b`和`&mut b`都指向了同一块内存区域，这个在rust里面是不被允许的。特别的是rust注意到`delta_b`将要持有`b`不可变引用一直到main结束，但是在main 这个scope里面我们却要尝试修改b， 当然这个在rust里面是不允许的。

&emsp;为了能够执行借用规则，编译器需要知道每个引用的生命周期，但是有些时候编译器无法判断，这个时候就需要人工的去添加这些注释了，rust也为开发人员提供了一些工具，例如我们可以要求在实现了某个trait的structure的所有的引用在给定的生命周期里面都是存活的。(引用和c++引用不是一个玩意，引用类似指针)


**Desugraing**
&emsp;在我们深入理解lifetime之前，我们首先需要明白lifetime是什么。在这里我们用`lifetime` 来指代一个作用域，用`lifetime-parameter`来表示一个参数，编译器会用一个真的lifetime来代替他。就像泛型一样。
例子：
```rust
use std::fmt::Display;


fn announce(value: &impl Display) {
    println!("Behold!{}", value);
}

fn announce_desugraeds<'a, T>(value: &'a T) where T: Display {
    println!("Behold!{}", value);
}

fn main(){
    /*
    let num = 42;
    let num_ref = &num;
    announce(num_ref);
    */
    'x: {
        let num = 42;
        'y:{
            let num_ref = &'y num;
            'z: {
                announce(num_ref);
            }
        }
    }
}
```
&emsp;上面的代码去除语法糖后，都标注了lifetime， 一个lifetime-parameter `'a` 和 两个lifetime/scopes `'x, 'y`和`'x1` ,同时来利用了泛型来和lifetime做对比。


**子类型**
&emsp;从技术上来看，lifetime并不是类型，因为我们无法去实力化它。然而当我们对函数或者结构体进行lifetime参数化的时候，他们看起来又像一个类型(就像上面例子那样). `variance rule`里面我们会看到他们不一样的地方，目前暂时当成类型来看。
&emsp;针对常规的类型和lifetime，liftime-parameter和常规类型参数做个对比还是很有用的：
- 当编译器在为一个常规类型做类型推断的时候，如果存在多个参数满足这个类型，那么编译器就会报错。但是针对lifetime，如果又多个lifetime参数满足给定的lifetime，那么编译器会选择最小的那个。
- rust简单的类型里面没有自类型，一个structure里面不能有另外一个structure类型，除非他们有lifetime-parameters。然而lifetime允许自类型，如果`long`的那个lifetime完全的cover住`short`的那个lifetime，那么`long`就是`short`的自类型。(rust的常规类型子类型个人理解，类似在go语言里面允许structure嵌套匿名结构体，但是在rust里不允许)


**Rule**
&emsp;针对rust的类型强制转换，rust有一套规则来限制。虽然强制转换和子类型非常相似，但是他们还是有很大的不同的。特别的时候强制类型转换的时候rust编译器会在强转的地方插入一些代码，但是在执行lifetime type的时候只是简单的做一些编译器检查工作。在底层加入的一些额外的代码这个对于开发人员来讲是不透明的。下面看下他们之间的区别：
```rust
fn main(){
    // coercion
    let values: [u32;5] = [1,2,3,4,5];
    let slice: &[u32] = &values;

    // subtyping
    let val1 = 42;
    let val2 = 24;
    'x: {
        let ref1 = &'x val1;
        'y: {
            let mut ref2 = &'y val2;
            ref2 = ref1;
        }
    }

}
```
&emsp;上面代码能工作是因为`'x`是`'y`的子类型，所以`&'x`也是`&'y`的子类型。

