extern crate rgraph;

use rgraph::*;
use std::vec::Vec;

fn main() {

    let mut g = Graph::new();

    g.add_node(create_node!(
                gen_one () -> (one: u32) {
                    println!("              gen one");
                    one = 1u32;
                }
            ));

    g.add_node(create_node!(
                plus_one (one: u32) -> (plusone : u32) {
                    println!("              plusone");
                    plusone = one + 1u32;
                }
            ));

    g.add_node(create_node!(
                the_one_task  (one: u32, plusone : u32) ->
                              (last_value: f32) {
                    println!("              the one task");
                    last_value = (one + plusone) as f32;
                }
            ));

    g.add_node(create_node!(
                list (list : Vec<u32>) -> () {
                    println!("             list");
                }
            ));

    g.bind_asset("gen_one :: one", "plus_one :: one")
        .expect("binding must be doable");
    g.bind_asset("gen_one :: one", "the_one_task :: one")
        .expect("binding must be doable");
    g.bind_asset("plus_one :: plusone", "the_one_task :: plusone")
        .expect("binding must be doable");

    let mut cache = ValuesCache::new();

    for _ in 0..10 {

        {
            let mut solver = GraphSolver::new(&g, &mut cache);
            assert!(solver.execute("nop").is_err());
            solver.execute("the_one_task").expect("could not execute");

            println!("{:?}", solver.get_values());

            assert!(solver
                        .get_value::<u32>("gen_one :: one")
                        .expect("never created?") == 1);
            assert!(solver
                        .get_value::<u32>("plus_one :: plusone")
                        .expect("never created?") == 2);
            assert!(solver
                        .get_value::<f32>("the_one_task :: last_value")
                        .expect("never created?") == 3.0);
        }

        assert!(cache
                    .get_value::<u32>("gen_one :: one")
                    .expect("not cached?") == 1);
        assert!(cache
                    .get_value::<u32>("plus_one :: plusone")
                    .expect("not cached?") == 2);
        assert!(cache
                    .get_value::<f32>("the_one_task :: last_value")
                    .expect("not cached?") == 3.0);

    }
}
