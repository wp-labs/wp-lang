pub trait New1<T> {
    fn new(args: T) -> Self;
}

pub trait New3<T1, T2, T3> {
    fn new(a1: T1, a2: T2, a3: T3) -> Self;
}

pub trait OptionConv<T, E> {
    fn no_less(self, name: &str) -> Result<T, E>;
    fn no_empty(self) -> Result<T, E>;
}

pub trait OptionError {
    fn empty() -> Self;
    fn less(msg: String) -> Self;
}
