

fn main(){
    let add = |x, y| x + y;
    receives_closure(add);
    receivers_closure_v1(Box::new(add));

    let y = 2;
    let add_on_parment = |x| x + y;
    receives_closure_one_parment(add_on_parment);


    {
        let y = 2;
        receives_closure_one_parment(|x| x + y);
    }
    {
        let y = 3;
        receives_closure_one_parment(|x| x + y);
    }

    let closure = returns_closures();
    println!("Closure(1) => {}", closure(1));
    receives_closure_one_parment(returns_closures());


}

fn curry<F> (f: F, x:i32) -> impl Fn(i32) -> i32 
where 
    F: Fn(i32, i32) -> i32,
{
    |y| f(x, y)
}

fn receives_closure<F>(closure: F) 
where 
    F: Fn(i32, i32) -> i32,
{
    let sum = closure(1,2);
    println!("sum is {}", sum);
}


fn receivers_closure_v1(closure: Box<dyn Fn(i32, i32) -> i32 >) {
    let sum = closure(1,2);
    println!("sum is {}", sum);
}


fn receives_closure_one_parment<F>(closure: F) 
where
    F: Fn(i32) -> i32,
{
    let result = closure(1);
    println!("closure(1) => {}", result);
}



fn returns_closures() -> impl Fn(i32) -> i32 {
    |x| x + 4
}



