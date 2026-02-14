use crate::components::{Appendable, IterativeReadable};
use bytemuck::Pod;
use std::cell::{Cell, RefCell};
use std::marker::PhantomData;

pub struct Window<InValue, OutValue = ()> {
    pub(crate) _v: PhantomData<InValue>,
    pub(crate) _out_v: PhantomData<OutValue>,
    pub(crate) last_index: Cell<usize>,
    pub(crate) buffer: RefCell<Vec<InValue>>,
}

impl<InValue, OutValue> Window<InValue, OutValue> {
    pub fn new() -> Window<InValue, OutValue> {
        Self {
            _v: PhantomData,
            _out_v: PhantomData,
            last_index: Cell::new(0),
            buffer: RefCell::new(Vec::new()),
        }
    }
}

impl<InValue, OutValue> Default for Window<InValue, OutValue> {
    fn default() -> Self {
        Self::new()
    }
}

impl<InValue: Pod + Send, OutValue: Pod + Send> Window<InValue, OutValue> {
    pub fn from<'a, R: IterativeReadable<InValue>>(
        &'a self,
        reader: &'a R,
    ) -> WindowFrom<'a, InValue, OutValue, R> {
        WindowFrom {
            window: self,
            reader,
            _in: PhantomData,
            _out_v: PhantomData,
        }
    }

    pub fn pipe(
        _source: impl IterativeReadable<InValue>,
        _target: impl Appendable<OutValue>,
    ) -> Self {
        Self::new()
    }
}

pub struct WindowFrom<'a, InValue: Pod + Send, OutValue: Pod + Send, R: IterativeReadable<InValue>>
{
    window: &'a Window<InValue, OutValue>,
    reader: &'a R,
    _in: PhantomData<InValue>,
    _out_v: PhantomData<OutValue>,
}

impl<'a, InValue: Pod + Send, OutValue: Pod + Send, R: IterativeReadable<InValue>>
    WindowFrom<'a, InValue, OutValue, R>
{
    pub fn to<'b, S: Appendable<OutValue>>(
        self,
        store: &'b mut S,
    ) -> WindowTo<'a, 'b, InValue, OutValue, R, S> {
        WindowTo {
            window: self.window,
            reader: self.reader,
            store,
            _in: PhantomData,
            _out: PhantomData,
        }
    }
}

pub struct WindowTo<
    'a,
    'b,
    InValue: Pod + Send,
    OutValue: Pod + Send,
    R: IterativeReadable<InValue>,
    S: Appendable<OutValue>,
> {
    window: &'a Window<InValue, OutValue>,
    reader: &'a R,
    store: &'b mut S,
    _in: PhantomData<InValue>,
    _out: PhantomData<OutValue>,
}

impl<'a, 'b, InValue, OutValue, R, S> WindowTo<'a, 'b, InValue, OutValue, R, S>
where
    InValue: Pod + Send,
    OutValue: Pod + Send,
    R: IterativeReadable<InValue>,
    S: Appendable<OutValue>,
{
    pub fn reduce(
        &mut self,
        window_size: u32,
        mut update_fn: impl FnMut(&[InValue]) -> Option<OutValue>,
    ) {
        let mut buffer = self.window.buffer.borrow_mut();
        let mut last_index = self.window.last_index.get();

        let current_index = self.reader.get_index();
        if current_index > last_index {
            if let Some(val) = self.reader.get() {
                buffer.push(val);
                if buffer.len() > window_size as usize {
                    buffer.remove(0);
                }

                if buffer.len() == window_size as usize
                    && let Some(out) = update_fn(&buffer)
                {
                    self.store.append(out);
                }
            }
            last_index = current_index;
            self.window.last_index.set(last_index);
        }
    }
}
