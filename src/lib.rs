//!
//!   The rgraph crate:
//!
//!   This library provides the mechanisms to define a directed acyclic graph of tasks.
//!   Once the graph is generated, a solver object can be instanciated to execute any of
//!   the tasks defined. In order to satisfy the input of such task, all the producer
//!   tasks will be executed as well.
//!
extern crate dot;

use std::collections::BTreeMap as Map;
use std::vec::Vec;
use std::any::Any;
use std::rc::Rc;
use std::mem;
use std::cmp;

#[macro_use]
mod macros;
mod printer;

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// helper trait that hides heterogenous tasks behind a common interface
pub trait NodeRunner {
    fn get_name(&self) -> &str;
    fn run(&self, solver: &mut GraphSolver) -> Result<SolverStatus, SolverError>;
    fn get_ins(&self) -> &[String];
    fn get_outs(&self) -> &[String];
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
    fn get_ins(&self) -> &[String] {
        &self.ins
    }
    fn get_outs(&self) -> &[String] {
        &self.outs
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

#[derive(Debug)]
pub enum GraphError {
    UndefinedAssetSlot(String),
    DisconnectedDependency,
}

/// The graph class itself.
/// It holds the static information about the tasks (Nodes) and how they
/// depend on each other by waiting on resources (Assets)
#[derive(Default)]
pub struct Graph {
    nodes: Map<String, Rc<NodeRunner>>,
    terminals: Vec<Rc<NodeRunner>>,
    whatprovides: Map<String, Rc<NodeRunner>>,
    bindings: Map<String, String>,
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
            self.whatprovides.insert(out.clone(), newnode.clone());
        }
        if newnode.as_ref().get_outs().is_empty() {
            self.terminals.push(newnode.clone());
        }

        self.nodes.insert(name, newnode);
    }

    pub fn get_node(&self, name: &str) -> Option<&NodeRunner> {
        let key: String = name.into();
        self.nodes.get(&key).map(|res| res.as_ref())
    }

    pub fn get_terminals(&self) -> &[Rc<NodeRunner>] {
        self.terminals.as_slice()
    }

    pub fn get_binding(&self, name: &str) -> Option<&String> {
        self.bindings.get(name)
    }

    pub fn bind_asset(&mut self, src: &str, sink: &str) -> Result<(), GraphError> {

        if !self.nodes
                .values()
                .find(|node| {
                          node.get_ins()
                              .iter()
                              .find(|name| name.as_str() == sink)
                              .is_some()
                      })
                .is_some() {
            return Err(GraphError::UndefinedAssetSlot(sink.into()));
        }

        if !self.nodes
                .values()
                .find(|node| {
                          node.get_outs()
                              .iter()
                              .find(|name| name.as_str() == src)
                              .is_some()
                      })
                .is_some() {
            return Err(GraphError::UndefinedAssetSlot(src.into()));
        }

        self.bindings.insert(sink.into(), src.into());
        Ok(())
    }

    pub fn what_provides(&self, asset: &str) -> Option<&NodeRunner> {
        let key: String = asset.into();
        self.whatprovides.get(&key).map(|res| res.as_ref())
    }

    pub fn validate(&self) -> Result<(), GraphError> {
        Ok(())
    }

    fn iter(&self) -> std::collections::btree_map::Iter<String, Rc<NodeRunner>> {
        self.nodes.iter()
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// type used to store results of executions and pass it to further solver instances
pub type ValuesCache = Map<String, Rc<Any>>;

pub trait Cache {
    /// Retrieves a value from the solver. It is required to know the
    /// name and type of the asset. Cast error will return SolveError::AssetWrongType
    fn get_value<T>(&self, name: &str) -> Result<T, SolverError> where T: Clone + 'static;

    /// Saves a value to be available during execution. This routine
    /// can be used to feed initial values for Assets. i.e. free Assets not
    /// generated by any Task.
    fn save_value<T>(&mut self, name: &str, value: T) where T: Clone + 'static;
}

impl Cache for ValuesCache {
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
    where T: cmp::PartialEq
{
    fn ne(&self, other: &Self) -> bool {
        self != other
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// The graph solver is a transient object which can execute the tasks described in a graph.
/// It is designed to be generated and droped on every execution.
pub struct GraphSolver<'a, 'b> {
    graph: &'a Graph,
    cache: ValuesCache,
    last_cache: &'b mut ValuesCache,
}

/// errors that may happen during a Solver instance execution
#[derive(Debug)]
pub enum SolverError {
    /// The asset was never declared during graph construction
    AssetNotDeclared(String),
    /// A node producing such asset was not declared
    AssetNotProduced(String),
    /// The asset was never instanciated during graph execution
    AssetNotCreated(String),
    /// The asset trying to retrieve is of a different type. Users of this interface
    /// meant to know the name and type of each asset.
    AssetWrongType(String),
    /// the asset in not bound, no connection can be found in the graph that satisfies this
    /// asset
    AssetUnbound(String),
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
    pub fn new(graph: &'a Graph, last_cache: &'b mut ValuesCache) -> GraphSolver<'a, 'b> {
        GraphSolver {
            graph: graph,
            cache: ValuesCache::new(),
            last_cache: last_cache,
        }
    }

    pub fn get_binding(&self, name: &str) -> Result<&String, SolverError> {
        match self.graph.get_binding(name) {
            Some(x) => Ok(x),
            None => Err(SolverError::AssetUnbound(name.into())),
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

    pub fn execute_terminals(&mut self) -> Result<SolverStatus, SolverError> {
        let tmp: Vec<&NodeRunner> = self.graph
            .get_terminals()
            .iter()
            .map(|x| x.as_ref())
            .collect();
        self.execute_all(tmp.as_slice())
    }

    fn execute_all(&mut self, nodes: &[&NodeRunner]) -> Result<SolverStatus, SolverError> {

        let mut queue = Vec::new();
        let mut to_run = Vec::new();

        for n in nodes {
            queue.push(*n);
        }

        while !queue.is_empty() {
            let node = queue.pop().unwrap();

            for input in node.get_ins() {
                match self.graph.get_binding(input) {
                    None => {
                        if !self.cache.contains_key(input) {
                            return Err(SolverError::AssetNotDeclared(input.clone()));
                        }
                    }
                    Some(producer_binding) => {
                        match self.graph.what_provides(producer_binding) {
                            Some(n) => queue.push(n),
                            None => {
                                return Err(SolverError::AssetNotProduced(producer_binding.clone()));
                            }
                        };
                    }
                }
            }

            to_run.push(node);
        }

        for node in to_run.iter().rev() {
            let _r = node.run(self)?;
        }

        Ok(SolverStatus::Executed)
    }

    /// check if the input is still valid. this function is used
    /// to compute if the input of a task has changed over iterations.
    /// if all inputs are cachedi and equal to current values, and a cached
    /// output is available. the output will be considered valid and the computation
    /// skiped
    pub fn input_is_new<T>(&self, new_value: &T, name: &str) -> bool
        where T: Clone + Comparable + 'static
    {
        // retrieve from last cache cache
        match self.last_cache.get_value::<T>(name) {
            Ok(old_value) => {
                //println!("values for {} differ? {}", name, new_value.ne(&old_value));
                new_value.ne(&old_value)
            }
            Err(_x) => {
                //println!("value not found in cache? {} {:?}", name, _x);
                true
            }
        }
    }

    /// function to decide whenever the set of values is still valid or the producing node of
    /// any of the values needs to be executed
    pub fn use_old_ouput(&mut self, ouputs: &Vec<&str>) -> bool {

        for out in ouputs {
            let name: String = (*out).into();
            if let Some(x) = self.last_cache.get(&name) {
                self.cache.insert(name, x.clone());
            } else {
                return false;
            }
        }
        true
    }

    pub fn get_values(&self) -> &ValuesCache {
        &self.cache
    }
}

impl<'a, 'b> Cache for GraphSolver<'a, 'b> {
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
    fn drop(&mut self) {
        mem::swap(&mut self.cache, &mut self.last_cache);
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

#[cfg(test)]
mod tests {
    use super::*;

    //#![macro_use]
    use macros::*;

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

        let mut cache = ValuesCache::new();
        let mut solver = GraphSolver::new(&g, &mut cache);

        let x = g.get_node("one").expect("must exist");
        assert!(x.run(&mut solver).is_ok());
        let y = g.get_node("two").expect("must exist");
        assert!(y.run(&mut solver).is_ok());
    }

    #[test]
    fn node_not_ready() {
        let g = Graph::new();
        let mut cache = ValuesCache::new();
        let mut solver = GraphSolver::new(&g, &mut cache);

        let node = create_node!(
            test (i : u32) -> (x: u32) {
                x = i +1;
            }
        );

        assert!(node.run(&mut solver).is_err());
    }

    #[test]
    fn construct_nodes() {

        let mut g = Graph::new();
        g.add_node(create_node!(no_output ( i : u32, j : u32) -> ()
                                 {
                                     println!("{} {}", i, j);
                                 }));

        g.add_node(create_node!(no_input  ( ) -> ( x : f32, y : f64)
                                 {
                                     x = 1.0f32;
                                     y = 4.0;
                                 }));

        g.add_node(create_node!(both_input_and_output ( i : u32, j : u32)
                                 -> ( x : f32, y : f64)
                                 {
                                     x = i as f32;
                                     y = j as f64;
                                 }));

    }

    #[test]
    fn solver() {

        let g = Graph::new();
        let mut cache = ValuesCache::new();
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
                gen_one () ->  (one: u32) {
                    println!("gen one");
                    one = 1u32;
                }
            ));

        g.add_node(create_node!(
                plus_one (one: u32) -> (plusone : u32) {
                    println!("plusone");
                    plusone = one + 1u32;
                }
            ));

        g.add_node(create_node!(
                the_one_task  (one: u32, plusone : u32) -> (last_value: f32) {
                    println!("the one task");
                    last_value = (one + plusone) as f32;
                }
            ));

        //do connection
        g.bind_asset("gen_one :: one", "plus_one :: one")
            .expect("binding must be doable");
        g.bind_asset("plus_one :: plusone", "the_one_task :: plusone")
            .expect("binding must be doable");
        g.bind_asset("gen_one :: one", "the_one_task :: one")
            .expect("binding must be doable");

        g.get_binding("plus_one :: one")
            .expect("binding must be set");
        g.get_binding("the_one_task :: plusone")
            .expect("binding must be set");
        g.get_binding("the_one_task :: one")
            .expect("binding must be set");

        let mut cache = ValuesCache::new();

        for _ in 0..10 {
            let mut solver = GraphSolver::new(&g, &mut cache);
            assert!(solver.execute("nop").is_err());
            solver.execute("the_one_task").expect("could not execute");
            solver
                .get_value::<f32>("the_one_task :: last_value")
                .expect("could not retrieve result");
        }

        assert!(cache
                    .get_value::<f32>("the_one_task :: last_value")
                    .expect("must be f32") == 3f32);
    }

    #[test]
    fn terminals() {

        let mut g = Graph::new();
        g.add_node(create_node!(sink_1 ( input : u32) -> ()
                                 {
                                     println!("sink 1 {}", input);
                                 }));

        g.add_node(create_node!(sink_2 ( name : u32) -> ()
                                 {
                                     println!("sink 2 {}", name);
                                 }));

        g.add_node(create_node!(no_input () -> ( i : u32)
                                 {
                                     i =  1234;
                                     println!("produce {}", i);
                                 }));

        g.bind_asset("no_input :: i", "sink_1 :: input")
            .expect("binding must be doable");
        g.bind_asset("no_input :: i", "sink_2 :: name")
            .expect("binding must be doable");

        let mut cache = ValuesCache::new();
        {
            let mut solver = GraphSolver::new(&g, &mut cache);
            solver.execute_terminals().expect("this should run");
        }
        assert!(cache
                    .get_value::<u32>("no_input :: i")
                    .expect("must be f32") == 1234);
    }

    #[test]
    fn graph_validate() {

        let g = Graph::new();
        assert!(g.validate().is_ok());
    }
}
