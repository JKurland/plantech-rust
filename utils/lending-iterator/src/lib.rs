use std::marker::PhantomData;


trait LendingIterator {
    type Item<'item> where Self: 'item;

    fn next<'item>(&'item mut self) -> Option<Self::Item<'item>>;


    fn map<'a, F: 'a>(self, f: F) -> Map<'a, Self, F>
    where
        F: for<'any> MyFnMut<'any, Self::Item<'any>>,
        Self: Sized
    {
        Map{i: self, f, _p: PhantomData}
    }

    fn take(self, n: usize) -> Take<Self> where Self: Sized {
        Take{i: self, rem: n}
    }
}


trait MyFnMut<'a, In> {
    type Output;

    fn invoke(&mut self, i: In) -> Self::Output;
}

impl<'a, In, Out, T> MyFnMut<'a, In> for T
where T: Fn(In) -> Out,
      Out: 'a
 {
    type Output = Out;

    fn invoke(&mut self, i: In) -> Self::Output {
        self(i)
    }
}


struct Map<'a, I, F>
where
    I: LendingIterator,
    F: for<'any> MyFnMut<'any, I::Item<'any>>,
    F: 'a
{
    i: I,
    f: F,
    _p: PhantomData<&'a()>
}

impl<'a, I, F> LendingIterator for Map<'a, I, F>
where
    I: LendingIterator,
    F: for<'any> MyFnMut<'any, I::Item<'any>>,
    F: 'a
{
    type Item<'item> = <F as MyFnMut<'item, I::Item<'item>>>::Output where Self: 'item;

    fn next<'item>(&'item mut self) -> Option<Self::Item<'item>> {
        match self.i.next() {
            Some(a) => Some(self.f.invoke(a)),
            None => None
        }
    }
}

struct Repeat<T> {
    item: T
}

impl<T> LendingIterator for Repeat<T> {
    type Item<'item> = &'item T where Self: 'item;

    fn next<'item>(&'item mut self) -> Option<Self::Item<'item>> {
        Some(&self.item)
    }
}


struct Take<I: LendingIterator> {
    i: I,
    rem: usize,
}

impl<I: LendingIterator> LendingIterator for Take<I> {
    type Item<'item> = I::Item<'item> where Self: 'item;

    fn next<'item>(&'item mut self) -> Option<Self::Item<'item>> {
        if self.rem > 0 {
            self.rem -= 1;
            self.i.next()
        } else {
            None
        }
    }
}


fn f() {
    let it = Repeat{item: [1,2,3,4]}.take(3).map(|x: &[i32; 4]| &x[1]);

}
