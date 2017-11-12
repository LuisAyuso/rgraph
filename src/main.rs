
#[macro_use]
mod mixlist;
use mixlist::*;

use std::mem;
use std::collections::BTreeMap as Map;
use std::vec::Vec;
use std::fmt::Debug;

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

trait NodeRunner{
    fn get_name(&self) -> &str;
    fn run(&self, &mut GraphSolver);
    fn get_ins(&self) -> &Vec<String>;
    fn get_outs(&self) -> &Vec<String>;
}

struct Node<F>
where F : Fn(&mut GraphSolver)
{
    name: String,
    func: F,
    ins: Vec<String>,
    outs: Vec<String>,
}

impl<F> Node<F>
where F : Fn(&mut GraphSolver)
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
where F : Fn(&mut GraphSolver)
{
    fn get_name(&self) -> &str{
        self.name.as_str()
    }
    fn run(&self, solver: &mut GraphSolver){
        (self.func)(solver);
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

    nodes: Map<String, Box<NodeRunner>>
}

impl Graph{

    pub fn new() -> Graph{
        Graph{
            nodes: Map::new(),
        }
    }

    pub fn add_node<F: 'static>(&mut self, node: Node<F>)
        where F : Fn(&mut GraphSolver)
    {
        let newnode = Box::new(node);
        let name : String = newnode.as_ref().get_name().into();
        self.nodes.insert(name, newnode);
    }

    pub fn get_node(&self, name: &str) -> Option<&NodeRunner>{
        let key : String = name.into();
        self.nodes.get(&key).map(|res| res.as_ref())
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

struct GraphSolver<'a>{
    graph: &'a Graph,
    cache: Map<String, *const u8>
}

impl<'a> GraphSolver<'a>{


    pub fn new(graph: &'a Graph) -> GraphSolver<'a>{
        GraphSolver{
            graph: graph,
            cache: Map::new(),
        }
    }

    pub fn get_value<T>(&self, name: &str) -> T
    where T: Debug + Clone 
    {
        let ptr = self.cache.get(name.into()).unwrap();
        let ptr : &Box<T> = unsafe { mem::transmute(ptr) };
        println!("get value for {} : {:?} ", name, *ptr);
        ptr.as_ref().clone()
    }

    pub fn save_value<T>(&mut self, name: &str, value: T)
        where T: Debug + Clone
    {
        println!("save value for {} {:?}", name, value);
        let ptr = Box::new(value);
        let ptr : *const u8 = unsafe { mem::transmute(ptr) };
        self.cache.insert(name.into(), ptr);
    }

    pub fn execute(&mut self, name: &str){

        let node = self.graph.get_node(name).unwrap();
        let requirements = node.get_ins();

        here, iterate requirements, find who does them. schedule them to execute

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
                let output = pack!(  $( $out ),+ );

                // save outputs
                $( let $out : $ot = $out; )+
                unpack!(output =>  $( $out : $ot ),+ );
                $( solver.save_value(stringify!($out), $out); )+
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
                $( let $in : $it = solver.get_value(stringify!($in)); )+

                // exec body
                $( let $out : $ot; )+
                $( $body )+
                let output = pack!(  $( $out ),+ );

                // save outputs
                $( let $out : $ot = $out; )+
                unpack!(output =>  $( $out : $ot ),+ );
                $( solver.save_value(stringify!($out), $out); )+
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
                $( let $in : $it = solver.get_value(stringify!($in)); )+

                // exec body
                $( $body )+
           }, 
           vec!( $( stringify!($in).to_string() ),+ ), 
           vec!()
       )
    };

);

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

fn main() {

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

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph() {

        let mut g = Graph::new();

        let node = Node::new("one", |solver| { 
            println!("stored and run 1");
        }, vec!(), vec!());
        g.add_node(node);

        let node = Node::new("two", |solver|{ 
            println!("stored and run 2");
        }, vec!(), vec!());
        g.add_node(node);

        let mut solver = GraphSolver::new(&g);

        let x = g.get_node("one").unwrap();
        x.run(&mut solver);
        let y = g.get_node("two").unwrap();
        y.run(&mut solver);
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

        solver.save_value("i", 1);
        node.run(&mut solver);
        let res : u32 = solver.get_value("x");
        assert!(res == 2);
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
    fn graph_with_assets() {

        let mut g = Graph::new();

        let node = create_node!( 
            name: "plus one",
            in: (i : u32), 
            out: (x: u32) {
                x = i +1;
            }
        );

        for _ in 0..10 {
            let mut solver = GraphSolver::new(&g);
            solver.save_value("i", 1);
        }
    }
}
