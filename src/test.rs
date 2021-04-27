

pub mod test {

    pub struct Bonk {
        pub v: i32,
    }

    pub mod meow {
        use super::Bonk;


        pub struct Cat {
            pub bonk: Bonk
        }

    }

    pub mod meow2 {
        use super::*;
    }
}