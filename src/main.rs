

use std::collections::BTreeMap as Map;
use std::vec::Vec;
use std::fmt::Debug;
use std::any::Any;


// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

trait NodeRunner{
    fn get_name(&self) -> &str;
    fn run(&self, solver: &mut GraphSolver) -> Result<(), SolverError>;
    fn get_ins(&self) -> &Vec<String>;
    fn get_outs(&self) -> &Vec<String>;
}

struct Node<F>
where F : Fn(&mut GraphSolver) -> Result<(), SolverError>
{
    name: String,
    func: F,
    ins: Vec<String>,
    outs: Vec<String>,
}

impl<F> Node<F>
where F : Fn(&mut GraphSolver) -> Result<(), SolverError>
{
    fn new(name: &str, func: F, ins: Vec<String>, outs: Vec<String>) -> Node<F>{
        Node{
            name: name.into(),
            func: func,
            ins: ins,
            outs: outs,
        }
    }
}

impl<F> NodeRunner for Node<F>
where F : Fn(&mut GraphSolver) -> Result<(), SolverError>
{
    fn get_name(&self) -> &str{
        self.name.as_str()
    }
    fn run(&self, solver: &mut GraphSolver) -> Result<(), SolverError>{
        (self.func)(solver)
    }
    fn get_ins(&self) -> &Vec<String>{
        &self.ins
    }
    fn get_outs(&self) -> &Vec<String>{
        &self.outs
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

struct Graph{
    nodes: Map<String, Box<NodeRunner>>,
    whatprovides: Map<String, String>,
}

impl Graph{

    pub fn new() -> Graph{
        Graph{
            nodes: Map::new(),
            whatprovides: Map::new(),
        }
    }

    pub fn add_node<F: 'static>(&mut self, node: Node<F>)
        where F : Fn(&mut GraphSolver) -> Result<(), SolverError>
    {
        let newnode = Box::new(node);
        let name : String = newnode.as_ref().get_name().into();

        for out in newnode.as_ref().get_outs(){
            self.whatprovides.insert(out.clone(), name.clone());
        }

        self.nodes.insert(name, newnode);
    }

    pub fn get_node(&self, name: &str) -> Option<&NodeRunner>{
        let key : String = name.into();
        self.nodes.get(&key).map(|res| res.as_ref())
    }

    pub fn what_provides(&self, name: &String) -> Option<&String>{
        self.whatprovides.get(name)
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

struct GraphSolver<'a>{
    graph: &'a Graph,
    cache: Map<String, Box<Any>>
}

#[derive(Debug)] 
enum SolverError{
    AssetNotDeclared(String),
    AssetNotCreated(String),
    AssetWrongType(String),
    NodeNotFound(String),
}

impl<'a> GraphSolver<'a>{

    pub fn new(graph: &'a Graph) -> GraphSolver<'a>{
        println!("new solver");
        GraphSolver{
            graph: graph,
            cache: Map::new(),
        }
    }

    pub fn get_value<T>(&self, name: &str) -> Result<T, SolverError>
    where T: Debug + Clone + 'static
    {
        if let Some(ptr) = self.cache.get(name.into()){
            if let Some(x) =  ptr.as_ref().downcast_ref::<T>(){
                return Ok(x.clone());
            }
            else {
                return Err(SolverError::AssetWrongType(name.into()));
            }
        }
        Err(SolverError::AssetNotCreated(name.into()))
    }

    pub fn save_value<T>(&mut self, name: &str, value: T)
        where T: Debug + Clone + 'static
    {
        println!("    save value for {} {:?}", name, value);
        let ptr : Box<Any> = Box::new(value);
        self.cache.insert(name.into(), ptr);
    }

    pub fn execute(&mut self, name: &str) -> Result<(), SolverError> {

        let mut queue = Vec::new();
        let mut to_run = Vec::new();

        let node = self.graph.get_node(name);
        if node.is_none(){
            return Err(SolverError::NodeNotFound(name.into()));

        }
        queue.push( node.unwrap() );

        while queue.len() != 0 {
            let node = queue.pop().unwrap();

            for input in node.get_ins(){
                match self.graph.what_provides(input){
                    None => { return Err(SolverError::AssetNotDeclared(input.clone())); },
                    Some(provider) => { 
                        match self.graph.get_node(provider){ 
                            Some(n) => queue.push(n),
                            None => { return Err(SolverError::NodeNotFound(provider.clone())); },
                        };
                    },
                }
            }

            to_run.push(node);
        }

        for node in to_run.iter().rev(){
            node.run(self)?;
        }

        Ok(())
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~


macro_rules! create_node(

    // no inputs, with outputs
    ( name: $name:expr,
      in: ( ) ,
      out: ( $( $out:ident : $ot:ty ),+ )  $( $body:stmt )+  ) => {
        Node::new($name, 
           | solver : &mut GraphSolver  |
           { 
                // exec body
                $( let $out : $ot; )+
                $( $body )+

                // save outputs
                $( let $out : $ot = $out; )+
                $( solver.save_value(stringify!($out), $out); )+

                Ok(())
           },
           vec!( ), 
           vec!( $( stringify!($out).to_string() ),+ ), 
        )
    };

    // with inputs, with outputs
    ( name: $name:expr,
      in: ( $( $in:ident : $it:ty ),+ ) ,
      out: ( $( $out:ident : $ot:ty ),+ )  $( $body:stmt )+  ) => {
        Node::new($name, 
           | solver : &mut GraphSolver  |
           { 
                // get inputs
                $( let $in : $it = solver.get_value::<$it>(stringify!($in))?; )+

                // exec body
                $( let $out : $ot; )+
                $( $body )+

                // save outputs
                $( let $out : $ot = $out; )+
                $( solver.save_value(stringify!($out), $out); )+

                Ok(())
           }, 
           vec!( $( stringify!($in).to_string() ),+ ), 
           vec!( $( stringify!($out).to_string() ),+ ), 
       )
    };

    // with inputs, no outputs
    ( name: $name:expr ,
      in: ( $( $in:ident : $it:ty ),+ ) ,
      out: ( )  $( $body:stmt )+  ) => {
        Node::new($name, 
           | solver : &mut GraphSolver  |
           { 
                // get inputs
                $( let $in : $it = solver.get_value(stringify!($in))?; )+

                // exec body
                $( $body )+

                Ok(())
           }, 
           vec!( $( stringify!($in).to_string() ),+ ), 
           vec!()
       )
    };

);

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

fn main() {
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph() {

        let mut g = Graph::new();

        let node = Node::new("one", |_solver| { 
            println!("stored and ran 1");
            Ok(())
        }, vec!(), vec!());
        g.add_node(node);

        let node = Node::new("two", |_solver|{ 
            println!("stored and ran 2");
            Ok(())
        }, vec!(), vec!());
        g.add_node(node);

        let mut solver = GraphSolver::new(&g);

        let x = g.get_node("one").expect("must exist");
        assert!(x.run(&mut solver).is_ok());
        let y = g.get_node("two").expect("must exist");
        assert!(y.run(&mut solver).is_ok());
    }

    #[test]
    fn node_with_assets() {

        let g = Graph::new();
        let mut solver = GraphSolver::new(&g);

        let node = create_node!( 
            name: "test",
            in: (i : u32), 
            out: (x: u32) {
                x = i +1;
            }
        );

        solver.save_value("i", 1u32);
        assert!(node.run(&mut solver).is_ok());
        let res : u32 = solver.get_value("x").unwrap();
        assert!(res == 2);
    }

    #[test]
    fn node_not_ready() {
        let g = Graph::new();
        let mut solver = GraphSolver::new(&g);

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
        g.add_node( create_node!(name: "no output",
                                 in: ( i : u32, j : u32),
                                 out: ()
                                 {
                                     println!("{} {}", i, j);
                                 }));

        g.add_node( create_node!(name: "no input",
                                 in: ( ),
                                 out: ( x : f32, y : f64)
                                 {
                                     x = 1.0f32;
                                     y = 4.0;
                                 }));

        g.add_node( create_node!(name: "both input and output",
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
        let mut s = GraphSolver::new(&g);

        let a : i32 = 1;
        s.save_value("a",a);

        assert!(s.get_value::<i32>("a").is_ok());
        assert!(s.get_value::<u32>("a").is_err());
        assert!(s.get_value::<u32>("j").is_err());

        println!(" hey" );
    }

    #[test]
    fn graph_with_assets() {

        let mut g = Graph::new();

        g.add_node( create_node!(
                name: "gen one",
                in: (), 
                out: (one: u32) {
                    println!("gen one");
                    one = 1u32;
                }
            ));

        g.add_node( create_node!(
                name: "plus one",
                in: (one: u32), 
                out: (plusone : u32) {
                    println!("plusone");
                    plusone = one + 1u32;
                }
            ));

        g.add_node( create_node!(
                name: "the one task",
                in: (one: u32, plusone : u32), 
                out: (last_value: f32) {
                    println!("the one task");
                    last_value = (one + plusone) as f32;
                }
            ));

        for _ in 0..10 {
            let mut solver = GraphSolver::new(&g);
            assert!(solver.execute("nop").is_err());
            assert!(solver.execute("the one task").is_ok());
            assert!(solver.get_value::<f32>("last_value").is_ok());
        }
    }
}
