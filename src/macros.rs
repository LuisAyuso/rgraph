#[macro_export]
macro_rules! asset_str(
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

#[macro_export]
macro_rules! asset_string(
    (as_str, $node:expr, $asset:ident) => {
        [$node.as_str(), stringify!($asset)].join("::")
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

    // name as expression allows to generate function names programatically
    ( name: $name:expr,
      ( $( $in:ident : $it:ty ),* ) ->
      ( $( $out:ident : $ot:ty ),* ) $( $body:stmt )+ ) => {
        {
            let tmp = $name.clone();
            Node::new($name.clone(),
               move | solver : &mut GraphSolver  |
               {
                    // get inputs
                    $(
                        let $in : $it = solver.get_value::<$it>(
                                            solver.get_binding(&asset_string!(as_str, tmp, $in))?
                                    )?;
                    )*

                    // if any of the inputs is new (or there are no imputs)
                    let eq = [ $( solver.input_is_new(&$in, &asset_string!(as_str, tmp, $in)) ),* ];
                    if !eq.iter().fold(false, |acum, b| acum || *b){
                        let tmp = tmp.clone();
                        let outs = vec!( $( asset_string!(as_str, tmp, $out) ),* );
                        if solver.use_old_ouput(&outs){
                            return Ok(SolverStatus::Cached);
                        }
                    }

                    // exec body (declare out vars, uninitalized)
                    $( let $out : $ot; )*
                    $( $body )+

                    // save outputs (re assign, this guarantees output type)
                    $( let $out : $ot = $out; )*
                    $( solver.save_value(&asset_string!(as_str, tmp, $out), $out); )*

                    // set the status to executed
                    Ok(SolverStatus::Executed)
               },
               vec!( $( asset_string!(as_str, $name.clone(), $in) ),* ),
               vec!( $( asset_string!(as_str, $name.clone(), $out) ),* ),
           )
        }
    };

    // no quotes in name, more function like
    ( $name:ident
      ( $( $in:ident : $it:ty ),* ) ->
      ( $( $out:ident : $ot:ty ),* ) $( $body:stmt )+ ) => {
        Node::new(stringify!($name).to_string(),
           move | solver : &mut GraphSolver  |
           {
                // get inputs
                $(
                    let $in : $it = solver.get_value::<$it>(
                                        solver.get_binding(asset_str!($name,$in))?
                                )?;
                )*

                // if any of the inputs is new (or there are no imputs)
                let eq = [ $( solver.input_is_new_str(&$in, asset_str!($name,$in)) ),* ];
                if !eq.iter().fold(false, |acum, b| acum || *b){
                    let outs : Vec<&'static str> = vec!( $( asset_str!($name,$out) ),* );
                    if solver.use_old_ouput(&outs){
                        return Ok(SolverStatus::Cached);
                    }
                }

                // exec body (declare out vars, uninitalized)
                $( let $out : $ot; )*
                $( $body )+

                // save outputs (re assign, this guarantees output type)
                $( let $out : $ot = $out; )*
                $( solver.save_value_str(asset_str!($name,$out), $out); )*

                // set the status to executed
                Ok(SolverStatus::Executed)
           },
           vec!( $( asset_str!($name, $in).to_string() ),* ),
           vec!( $( asset_str!($name, $out).to_string() ),* ),
       )
    };
);

#[cfg(test)]
mod tests {

    #[test]
    fn names() {
        assert!(asset_str!(one, two) == "one::two");
        assert!(asset_str!("one", two) == "one::two");
        assert!(asset_str!("one", "two") == "one::two");
    }
}
