//!
//!   The rgraph crate:
//!
//!   This library provides the mechanisms to define a directed acyclic graph of tasks.
//!   Once the graph is generated, a solver object can be instanciated to execute any of
//!   the tasks defined. In order to satisfy the input of such task, all the producer
//!   tasks will be executed as well.
//!

use std::collections::BTreeMap as Map;
use std::vec::Vec;
use std::any::Any;
use std::rc::Rc;
use std::mem;
use std::cmp;

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// helper trait that hides heterogenous tasks behind a common interface
pub trait NodeRunner {
    fn get_name(&self) -> &str;
    fn run(&self, solver: &mut GraphSolver) -> Result<SolverStatus, SolverError>;
    fn get_ins(&self) -> &Vec<String>;
    fn get_outs(&self) -> &Vec<String>;
}

/// Generic that stores the information required to execute arbitrary tasks
/// Please use `create_node` macro to instaciate this objects
pub struct Node<F>
    where F: Fn(&mut GraphSolver) -> Result<SolverStatus, SolverError>
{
    name: String,
    func: F,
    ins: Vec<String>,
    outs: Vec<String>,
}

impl<F> Node<F>
    where F: Fn(&mut GraphSolver) -> Result<SolverStatus, SolverError>
{
    pub fn new(name: &str, func: F, ins: Vec<String>, outs: Vec<String>) -> Node<F> {
        Node {
            name: name.into(),
            func: func,
            ins: ins,
            outs: outs,
        }
    }
}

impl<F> NodeRunner for Node<F>
    where F: Fn(&mut GraphSolver) -> Result<SolverStatus, SolverError>
{
    fn get_name(&self) -> &str {
        self.name.as_str()
    }
    fn run(&self, solver: &mut GraphSolver) -> Result<SolverStatus, SolverError> {
        (self.func)(solver)
    }
    fn get_ins(&self) -> &Vec<String> {
        &self.ins
    }
    fn get_outs(&self) -> &Vec<String> {
        &self.outs
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// The graph class itself.
/// It holds the static information about the tasks (Nodes) and how they
/// depend on each other by waiting on resources (Assets)
#[derive(Default)]
pub struct Graph {
    nodes: Map<String, Rc<NodeRunner>>,
    sinks: Vec<Rc<NodeRunner>>,
    whatprovides: Map<String, String>,
}

impl Graph {
    pub fn new() -> Graph {
        Graph { ..Default::default() }
    }

    pub fn add_node<F: 'static>(&mut self, node: Node<F>)
        where F: Fn(&mut GraphSolver) -> Result<SolverStatus, SolverError>
    {
        let newnode = Rc::new(node);
        let name: String = newnode.as_ref().get_name().into();

        for out in newnode.as_ref().get_outs() {
            self.whatprovides.insert(out.clone(), name.clone());
        }
        if newnode.as_ref().get_outs().is_empty(){
            self.sinks.push(newnode.clone());
        }

        self.nodes.insert(name, newnode);
    }

    pub fn get_node(&self, name: &str) -> Option<&NodeRunner> {
        let key: String = name.into();
        self.nodes.get(&key).map(|res| res.as_ref())
    }

    pub fn get_sinks(&self) -> &[Rc<NodeRunner>]
    {
        self.sinks.as_slice()
    }

    pub fn what_provides(&self, name: &str) -> Option<&String> {
        self.whatprovides.get(name)
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// type used to store results of executions and pass it to further solver instances
pub type Cache = Map<String, Rc<Any>>;

pub trait CacheImpl{

    /// Retrieves a value from the solver. It is required to know the
    /// name and type of the asset. Cast error will return SolveError::AssetWrongType
    fn get_value<T>(&self, name: &str) -> Result<T, SolverError>
        where T: Clone + 'static;

    /// Saves a value to be available during execution. This routine
    /// can be used to feed initial values for Assets. i.e. free Assets not
    /// generated by any Task.
    fn save_value<T>(&mut self, name: &str, value: T)
        where T: Clone + 'static;

}

impl CacheImpl for Cache{

    fn get_value<T>(&self, name: &str) -> Result<T, SolverError>
        where T: Clone + 'static
    {
        if let Some(ptr) = self.get(name.into()) {
            if let Some(x) = ptr.as_ref().downcast_ref::<T>() {
                return Ok(x.clone());
            } else {
                return Err(SolverError::AssetWrongType(name.into()));
            }
        }
        Err(SolverError::AssetNotCreated(name.into()))
    }

    fn save_value<T>(&mut self, name: &str, value: T)
        where T: Clone + 'static
    {
        let ptr: Rc<Any> = Rc::new(value);
        self.insert(name.into(), ptr);
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// this trait allows us to overload behaviour for custom types
/// in this manner comparison can be optimized or bypased for
/// custom types
pub trait Comparable {
    fn ne(&self, other: &Self) -> bool;
}

impl<T> Comparable for T
where T : cmp::PartialEq{
    fn ne(&self, other: &Self) -> bool{
        self != other
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// The graph solver is a transient object which can execute the tasks described in a graph.
/// It is designed to be generated and droped on every execution.
pub struct GraphSolver<'a, 'b> {
    graph: &'a Graph,
    cache: Cache,
    last_cache: &'b mut Cache,
}

/// errors that may happen during a Solver instance execution
#[derive(Debug)]
pub enum SolverError {
    /// The asset was never declared during graph construction
    AssetNotDeclared(String),
    /// The asset was never instanciated during graph execution
    AssetNotCreated(String),
    /// The asset trying to retrieve is of a different type. Users of this interface
    /// meant to know the name and type of each asset.
    AssetWrongType(String),
    /// No node was found with this name.
    NodeNotFound(String),
}

#[derive(Debug)]
pub enum SolverStatus {
    Cached,
    Executed,
}

impl<'a, 'b> GraphSolver<'a, 'b> {

    /// creates a solver for graph 'graph', using cache from a previous solve. 
    /// the cache may be empty.
    pub fn new(graph: &'a Graph, last_cache: &'b mut Cache) -> GraphSolver<'a, 'b> {
        GraphSolver {
            graph: graph,
            cache: Map::new(),
            last_cache: last_cache,
        }
    }

    /// Executes a task by name, all tasks needed to provide Assets
    /// are transitively executed
    pub fn execute(&mut self, name: &str) -> Result<SolverStatus, SolverError> {

        let node = self.graph.get_node(name);
        if node.is_none() {
            return Err(SolverError::NodeNotFound(name.into()));

        }
        self.execute_all(&[node.unwrap()])
    }

    pub fn execute_sinks(&mut self) -> Result<SolverStatus, SolverError> {
        let tmp : Vec<&NodeRunner> = self.graph.get_sinks().iter().map(|x| x.as_ref() ).collect();
        self.execute_all(tmp.as_slice())
    }

    fn execute_all(&mut self, nodes: &[&NodeRunner]) -> Result<SolverStatus, SolverError> {

        let mut queue = Vec::new();
        let mut to_run = Vec::new();

        for n in nodes{
            queue.push(*n);
        }

        while !queue.is_empty() {
            let node = queue.pop().unwrap();

            for input in node.get_ins() {
                match self.graph.what_provides(input) {
                    None => {
                        if !self.cache.contains_key(input) {
                            return Err(SolverError::AssetNotDeclared(input.clone()));
                        }
                    }
                    Some(provider) => {
                        match self.graph.get_node(provider) {
                            Some(n) => queue.push(n),
                            None => {
                                return Err(SolverError::NodeNotFound(provider.clone()));
                            }
                        };
                    }
                }
            }

            to_run.push(node);
        }

        for node in to_run.iter().rev() {
            let _r = node.run(self)?;
            //println!("task: {} -> {:?}", node.get_name(), _r);
        }

        Ok(SolverStatus::Executed)
    }

    /// check if the input is still valid. this function is used
    /// to compute if the input of a task has changed over iterations.
    /// if all inputs are cachedi and equal to current values, and a cached 
    /// output is available. the output will be considered valid and the computation
    /// skiped
    pub fn input_is_new<T> (&self, new_value: &T, name: &str) -> bool
        where T : Clone + Comparable + 'static
    {
        // retrieve from last cache cache
        match self.last_cache.get_value::<T>(name){
            Ok(old_value) => {
                //println!("values for {} differ? {}", name, new_value.ne(&old_value));
                new_value.ne(&old_value)
            },
            Err(_x) =>{
                //println!("value not found in cache? {} {:?}", name, _x);
                true
            }
        }
    }
    pub fn use_old_ouput (&mut self, ouputs: &Vec<&str>) -> bool{

        for out in ouputs{
            let name : String = (*out).into();
            if let Some(x) = self.last_cache.get(&name){
                self.cache.insert(name, x.clone());
            }
            else{
                return false;
            }
        }
        true
    }
}

impl<'a, 'b> CacheImpl for GraphSolver<'a, 'b> {

    fn get_value<T>(&self, name: &str) -> Result<T, SolverError>
        where T: Clone + 'static
    {
        if let Some(ptr) = self.cache.get(name.into()) {
            if let Some(x) = ptr.as_ref().downcast_ref::<T>() {
                return Ok(x.clone());
            } else {
                return Err(SolverError::AssetWrongType(name.into()));
            }
        }
        Err(SolverError::AssetNotCreated(name.into()))
    }

    fn save_value<T>(&mut self, name: &str, value: T)
        where T: Clone + 'static
    {
        let ptr: Rc<Any> = Rc::new(value);
        self.cache.insert(name.into(), ptr);
    }
}

impl<'a, 'b> Drop for GraphSolver<'a, 'b> {
    fn drop(&mut self){
        mem::swap(&mut self.cache, &mut self.last_cache);
    }
}


// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Macro to generate a Node (Task).
/// It requires:
///   a name (as used in the solver to execute it),
///   a set of inputs,
///   a set of outputs, and
///   a set of statements which are the body of the task
#[macro_export]
macro_rules! create_node(

    // with inputs, with outputs
    ( name: $name:expr,
      in: ( $( $in:ident : $it:ty ),* ) ,
      out: ( $( $out:ident : $ot:ty ),* )  $( $body:stmt )+  ) => {
        Node::new($name,
           move | solver : &mut GraphSolver  |
           {
                // get inputs
                $( let $in : $it = solver.get_value::<$it>(stringify!($in))?; )*
                let eq = vec!( $( solver.input_is_new(&$in, stringify!($in)) ),* );
                // if any of the inputs is new (or there are no imputs)
                if !eq.iter().fold(false, |acum, b| acum || *b){
                    let outs = vec!( $( stringify!($out) ),* );
                    if solver.use_old_ouput(&outs){
                        return Ok(SolverStatus::Cached);
                    }
                }

                // exec body
                $( let $out : $ot; )*
                $( $body )+

                // save outputs
                $( let $out : $ot = $out; )*
                $( solver.save_value(stringify!($out), $out); )*

                Ok(SolverStatus::Executed)
           },
           vec!( $( stringify!($in).to_string() ),* ),
           vec!( $( stringify!($out).to_string() ),* ),
       )
    };
);

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph() {

        let mut g = Graph::new();

        let node = Node::new("one",
                             |_solver| {
                                 println!("stored and ran 1");
                                 Ok(SolverStatus::Executed)
                             },
                             vec![],
                             vec![]);
        g.add_node(node);

        let node = Node::new("two",
                             |_solver| {
                                 println!("stored and ran 2");
                                 Ok(SolverStatus::Executed)
                             },
                             vec![],
                             vec![]);
        g.add_node(node);

        let mut cache = Cache::new();
        let mut solver = GraphSolver::new(&g, &mut cache);

        let x = g.get_node("one").expect("must exist");
        assert!(x.run(&mut solver).is_ok());
        let y = g.get_node("two").expect("must exist");
        assert!(y.run(&mut solver).is_ok());
    }

    #[test]
    fn node_with_assets() {

        let g = Graph::new();
        let mut cache = Cache::new();
        let mut solver = GraphSolver::new(&g, &mut cache);

        let node = create_node!(
            name: "test",
            in: (i : u32),
            out: (x: u32) {
                x = i +1;
            }
        );

        solver.save_value("i", 1u32);
        assert!(node.run(&mut solver).is_ok());
        let res: u32 = solver.get_value("x").unwrap();
        assert!(res == 2);
    }

    #[test]
    fn node_not_ready() {
        let g = Graph::new();
        let mut cache = Cache::new();
        let mut solver = GraphSolver::new(&g, &mut cache);

        let node = create_node!(
            name: "test",
            in: (i : u32),
            out: (x: u32) {
                x = i +1;
            }
        );

        assert!(node.run(&mut solver).is_err());
    }

    #[test]
    fn construct_nodes() {

        let mut g = Graph::new();
        g.add_node(create_node!(name: "no output",
                                 in: ( i : u32, j : u32),
                                 out: ()
                                 {
                                     println!("{} {}", i, j);
                                 }));

        g.add_node(create_node!(name: "no input",
                                 in: ( ),
                                 out: ( x : f32, y : f64)
                                 {
                                     x = 1.0f32;
                                     y = 4.0;
                                 }));

        g.add_node(create_node!(name: "both input and output",
                                 in: ( i : u32, j : u32),
                                 out: ( x : f32, y : f64)
                                 {
                                     x = i as f32;
                                     y = j as f64;
                                 }));

    }

    #[test]
    fn solver() {

        let g = Graph::new();
        let mut cache = Cache::new();
        let mut s = GraphSolver::new(&g, &mut cache);

        let a: i32 = 1;
        s.save_value("a", a);

        assert!(s.get_value::<i32>("a").is_ok());
        assert!(s.get_value::<u32>("a").is_err());
        assert!(s.get_value::<u32>("j").is_err());

        println!(" hey");
    }

    #[test]
    fn graph_with_assets() {

        let mut g = Graph::new();

        g.add_node(create_node!(
                name: "gen one",
                in: (),
                out: (one: u32) {
                    println!("gen one");
                    one = 1u32;
                }
            ));

        g.add_node(create_node!(
                name: "plus one",
                in: (one: u32),
                out: (plusone : u32) {
                    println!("plusone");
                    plusone = one + 1u32;
                }
            ));

        g.add_node(create_node!(
                name: "the one task",
                in: (one: u32, plusone : u32),
                out: (last_value: f32) {
                    println!("the one task");
                    last_value = (one + plusone) as f32;
                }
            ));

        let mut cache = Cache::new();

        for _ in 0..10 {
            let mut solver = GraphSolver::new(&g, &mut cache);
            assert!(solver.execute("nop").is_err());
            assert!(solver.execute("the one task").is_ok());
            assert!(solver.get_value::<f32>("last_value").is_ok());
        }

        assert!(cache.get_value::<f32>("last_value").expect("must be f32") == 3f32);
    }

    #[test]
    fn sinks(){

        let mut g = Graph::new();
        g.add_node(create_node!(name: "sink 1",
                                 in: ( i : u32),
                                 out: ()
                                 {
                                     println!("sink 1 {}", i);
                                 }));

        g.add_node(create_node!(name: "sink 2",
                                 in: ( i : u32),
                                 out: ()
                                 {
                                     println!("sink 2 {}", i);
                                 }));

        g.add_node(create_node!(name: "no input",
                                 in: ( ),
                                 out: ( i : u32)
                                 {
                                     i =  1234;
                                     println!("produce {}", i);
                                 }));

        let mut cache = Cache::new();
        {
            let mut solver = GraphSolver::new(&g, &mut cache);
            solver.execute_sinks().expect("this should run");
        }
        assert!(cache.get_value::<u32>("i").expect("must be f32") == 1234);
    }
}
