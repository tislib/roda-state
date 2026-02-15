#[macro_export]
macro_rules! pipe {
    ($s1:expr) => { $s1 };
    ($s1:expr, $($rest:expr),+ $(,)?) => {
        {
            use $crate::StageExt;
            $s1.pipe($crate::pipe!($($rest),+))
        }
    };
}
