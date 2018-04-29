extern crate rgraph;

use rgraph::printer;
use rgraph::*;

fn main() {
    let mut g = Graph::new();

    let max = 10000;

    // generate 10000 nodes
    for i in 1..max {
        let name: String = format!("task{}", i);
        g.add_node(create_node!(name: name, ( input : u32) -> (output : u32)
                                 { 
                                     output = input +1 ;
                                 }))
            .unwrap();
    }


    // add sequential linking
    for i in 1..max - 1 {
        let src = format!("task{}::output", i);
        let sink = format!("task{}::input", i + 1);
        g.bind_asset(src.as_str(), sink.as_str())
            .expect("binding must be doable");
    }

    g.define_freestanding_asset("start", 0u32).expect("could not create asset");
        g.bind_asset("start", "task1::input")
            .expect("could not bind first tast to start value");

    let mut cache = ValuesCache::new();
    let mut solver = GraphSolver::new(&g, &mut cache);

    let last_task = format!("task{}", max-1);
    solver.execute(last_task.as_str()).expect("this should run");
}
