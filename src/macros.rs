
#[macro_export]
macro_rules! asset_name(
    ($node:ident, $asset:ident) => {
        concat!(stringify!($node), "::", stringify!($asset))
    };
    ($node:expr, $asset:ident) => {
        concat!($node, "::", stringify!($asset))
    };
    ($node:expr, $asset:expr) => {
        concat!($node, "::", $asset)
    };
);

/// Macro to generate a Node (Task).
/// It requires:
///   a name (as used in the solver to execute it),
///   a set of inputs,
///   a set of outputs, and
///   a set of statements which are the body of the task
#[macro_export]
macro_rules! create_node(

    // with inputs, with outputs
    ( $name:ident
      ( $( $in:ident : $it:ty ),* ) ->
      ( $( $out:ident : $ot:ty ),* ) $( $body:stmt )+ ) => {
        Node::new(stringify!($name),
           move | solver : &mut GraphSolver  |
           {
                // get inputs
                $( 
                    let $in : $it = solver.get_value::<$it>(
                                        solver.get_binding(asset_name!($name,$in))?
                                )?;
                )*

                // if any of the inputs is new (or there are no imputs)
                let eq = [ $( solver.input_is_new(&$in, asset_name!($name,$in)) ),* ];
                if !eq.iter().fold(false, |acum, b| acum || *b){
                    let outs = vec!( $( asset_name!($name,$out) ),* );
                    if solver.use_old_ouput(&outs){
                        return Ok(SolverStatus::Cached);
                    }
                }

                // exec body (declare out vars, uninitalized)
                $( let $out : $ot; )*
                $( $body )+

                // save outputs (re assign, this guarantees output type)
                $( let $out : $ot = $out; )*
                $( solver.save_value(asset_name!($name,$out), $out); )*

                // set the status to executed
                Ok(SolverStatus::Executed)
           },
           vec!( $( asset_name!($name,$in).to_string() ),* ),
           vec!( $( asset_name!($name,$out).to_string() ),* ),
       )
    };
);

#[cfg(test)]
mod tests {

    #[test]
    fn names() {
        assert!(asset_name!(one, two) == "one::two");
        assert!(asset_name!("one", two) == "one::two");
        assert!(asset_name!("one", "two") == "one::two");
    }
}
