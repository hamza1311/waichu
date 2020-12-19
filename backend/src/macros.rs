pub const ANYHOW_RESULT_TYPE_NAME: &str = "anyhow::Error>";
pub const RESULT_TYPE_NAME: &str = "core::result::Result";

#[macro_export]
macro_rules! setup_rejection {
    ($err:ident $code_ident: ident $message: ident $($path:path, $code:expr);*) => {
        if let Some(_) = Option::<&str>::None {}
        $(
            else if let Some(e) = $err.find::<$path>() {
                $code_ident = $code;
                $message = e.to_string();
            }
        )+
    };
}

#[macro_export]
macro_rules! bail_if_err {
    ($res:expr) => {{
        let result = $res
            .map_err(crate::utils::from_anyhow)
            .map_err(|e| e.into_response());

        match result {
            Ok(value) => value,
            Err(e) => return Ok(e),
        }
    }};
}

#[macro_export]
macro_rules! value_or_404 {
    ($expr:expr, $message:expr) => {{
        match $expr {
            Some(value) => value,
            None => {
                return Ok(warp::reply::with_status(
                    warp::reply::json(
                        &http_api_problem::HttpApiProblem::new($message)
                            .set_status(warp::http::StatusCode::NOT_FOUND),
                    ),
                    warp::http::StatusCode::NOT_FOUND,
                )
                .into_response())
            }
        }
    }};
    ($expr:expr) => {{
        crate::value_or_404!($expr, "Requested resource not found")
    }};
}

#[macro_export]
macro_rules! bail_if_err_or_404 {
    ($res:expr, $message:expr) => {{
        let value = crate::bail_if_err!($res);
        let value = crate::value_or_404!(value, $message);
        value
    }};
    ($res:expr) => {{
        let value = crate::bail_if_err!($res);
        let value = crate::value_or_404!(value);
        value
    }};
}

#[cfg(debug_assertions)]
macro_rules! debug_boxed {
    ($x:expr) => {
        ::warp::Filter::boxed($x)
    };
}

#[cfg(not(debug_assertions))]
macro_rules! debug_boxed {
    ($x:expr) => {
        $x
    };
}

/// Takes a list of handler expressions and `or`s them together
/// in a balanced tree. That is, instead of `a.or(b).or(c).or(d)`,
/// it produces `(a.or(b)).or(c.or(d))`, thus nesting the types
/// less deeply, which provides improvements in compile time.
///
/// It also applies `::warp::Filter::boxed` to each handler expression
/// when in `debug_assertions` mode, improving compile time further.
//
// The basic list splitting algorithm here is based on this gist:
// https://gist.github.com/durka/9fc479de2555225a787f
// It uses a counter from which two items are removed each time,
// stopping when the counter reaches 0. At each step, one item
// is moved from the left to the right, and thus at the end,
// there will be the same number of items in each list.
//
// The flow is as follows:
// - If there is one handler expression, debug_box it and return.
// - If there is more than one handler expression:
//   - First, copy the list into two: the one that will go into the
//     right side of the `or`, and one that will serve as a counter.
//     Recurse with these separated by semicolons, plus an empty `left`
//     list before the first semicolon.
//   - Then, as long as there are at least two items in the counter
//     list, remove them and move the first item on the right side of
//     the first semicolon (`head`) to the left side of the first semicolon.
//   - Finally, when there are one or zero items left in the counter,
//     move one last item to the left, make the call this macro on both the
//     left and right sides, and `or` the two sides together.
//
// For example, balanced_or_tree!(a, b, c, d, e) would take the following steps:
//
// - balanced_or_tree!(a, b, c, d, e)
// - balanced_or_tree!(@internal ; a, b, c, d, e ; a, b, c, d, e) // initialise lists
// - balanced_or_tree!(@internal a ; b, c, d, e ; c, d, e) // move one elem; remove two
// - balanced_or_tree!(@internal a, b ; c, d, e ; e) // now only one elem in counter
// - balanced_or_tree!(a, b, c).or(balanced_or_tree(d, e)) // recurse on each sublist
// From: https://github.com/seanmonstar/warp/issues/619#issuecomment-662716377
macro_rules! balanced_or_tree {
    // Base case: just a single expression, return it wrapped in `debug_boxed`
    ($x:expr $(,)?) => { debug_boxed!($x) };
    // Multiple expressions: recurse with three lists: left, right and counter.
    ($($x:expr),+ $(,)?) => {
        balanced_or_tree!(@internal  ;     $($x),+; $($x),+)
        //                          ^ left ^ right  ^ counter
    };
    // Counter 1 or 2; move one more item and recurse on each sublist, and or them together
    (@internal $($left:expr),*; $head:expr, $($tail:expr),+; $a:expr $(,$b:expr)?) => {
        (balanced_or_tree!($($left,)* $head)).or(balanced_or_tree!($($tail),+))
    };
    // Counter > 2; move one item from the right to the left and subtract two from the counter
    (@internal $($left:expr),*; $head:expr, $($tail:expr),+; $a:expr, $b:expr, $($more:expr),+) => {
        balanced_or_tree!(@internal $($left,)* $head; $($tail),+; $($more),+)
    };
}
