use std::collections::BTreeSet;


#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Me<'a>{
    my_holder: &'a Holder<'a>,
}


#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Holder<'a> {
    set_of_me: BTreeSet<&'a Me<'a>>,
}

impl Me<'a> {
    fn new(my_holder: &'a mut Holder<'a>) -> Self {
        let this = Self{
            my_holder,
        };
        my_holder.set_of_me.insert(&this);

        this
    }
}

fn main() {
    println!("Hello, world!");
}
