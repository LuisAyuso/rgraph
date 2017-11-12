// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

#[derive(Debug, Default)] 
pub struct End;

#[derive(Debug, Default)] 
pub struct Chain<X,T>(X,T);

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

pub trait Push<X> {
    type Next;
    fn push(self, other: X) -> Self::Next;
}

impl<X> Push<X> for () {
    type Next = Chain<X, End>;

    fn push(self, newelem: X) -> Self::Next {
        Chain(newelem, End)
    }
}

impl<T1, T2, X> Push<X> for Chain<T1,T2> {
    type Next = Chain<X, Chain<T1, T2>>;

    fn push(self, newelem: X) -> Self::Next {
        Chain(newelem, self)
    }
}

pub trait Pop<X> {
    type Next;
    fn pop(self) -> (X, Self::Next);
}

impl<T1, X> Pop<X> for Chain<X, T1> {
    type Next = T1;

    fn pop(self) -> (X, Self::Next){
        (self.0, self.1)
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

#[macro_export]
macro_rules! pack {

    ($e:expr) => {
        ().push($e)
    };
    ($e:expr, $($es:expr),+ ) => {
        pack!($($es),+).push($e)
    };
}

#[macro_export]
macro_rules! unpack {
    ( $list:expr => $($v:ident : $t:ty  ),* ) => {
        let tail = $list;
        $(
            let (tmp, tail) = tail.pop();
            let $v : $t = tmp;
        )*
        let _ = tail;
    };
}

#[macro_export]
macro_rules! decl_list{
    ($t:ty) => { Chain<$t, End> };
    ($t:ty, $($ts:ty),+) => { Chain< $t , decl_list!($($ts),*)  > };
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn mix_list_single(){
        let x = ().push(1);
        println!("{:?}", x);
        let y = x.push(1);
        println!("{:?}", y);
    }

    #[test]
    fn mix_list(){
        let x = ().push(1).push(true);
        println!("{:?}", x);
        let y = x.push(3.4);
        println!("{:?}", y);

        let (one, tail) = y.pop();
        assert!(one == 3.4);
        let (two, tail) = tail.pop();
        assert!(two == true);
        let (three, _tail) = tail.pop();
        assert!(three == 1);
    }

    #[test]
    fn unpack(){
        {
            let list = ();
            unpack!(list => );
        }
        {
            let list = ().push(1);
            unpack!(list => x : u32);
            assert!(x == 1);
        }
        {
            let list = ().push(1).push(true);
            unpack!(list => x : bool,
                            y : i32);
            assert!(x == true);
            assert!(y == 1);
        }
    }

    #[test]
    fn pack(){
        {
            let list = pack!(1);
            unpack!(list => x : u32);
            assert!(x == 1);
        }
        {
            let list = pack!(1, true, 3.5);
            unpack!(list => 
                        x : u32,
                        y : bool,
                        z : f32);
            assert!(x == 1);
            assert!(y == true);
            assert!(z == 3.5);
        }
    }

    #[test]
    fn define(){

        type A = decl_list!(u32);
        type B = decl_list!(u32, f32);
        type C = decl_list!(u32, f32, bool);

        {
            let list : decl_list!(i32) = Default::default();
            unpack!(list =>  a: i32);
            assert!(a == i32::default());
        }
        {
            let list : decl_list!(i32, bool) = Default::default();
            unpack!(list =>  a: i32, b: bool);
            assert!(a == i32::default());
            assert!(b == bool::default());
        }
        {
            use std::vec;
            type V = vec::Vec<u32>;
            let list : decl_list!(i32, bool, V) = Default::default();
            unpack!(list =>  a: i32, b: bool, c : V);
            assert!(a == i32::default());
            assert!(b == bool::default());
            assert!(c == V::default());
        }

    }
}
