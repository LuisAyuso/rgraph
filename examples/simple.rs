extern crate rgraph;

use rgraph::*;

fn main(){

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

        for _ in 0..10 {

            let mut solver = GraphSolver::new(&g);
            assert!(solver.execute("nop").is_err());
            assert!(solver.execute("the one task").is_ok());
            assert!(solver.get_value::<f32>("last_value").is_ok());

        }
}
