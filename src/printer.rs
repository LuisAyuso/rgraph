#![macro_use]

use super::*;

/// Prints a basic layout of nodes declared and assets they use
pub fn print_info(graph: &Graph) {
    for (name, node) in graph.iter() {
        print!("node: {} (", name);
        for input in node.get_ins() {
            print!("{},", input);
        }
        print!(") -> (");
        for output in node.get_outs() {
            print!("{},", output);
        }
        println!(")");
    }
}

mod topo {

    use super::*;

    use std::collections::BTreeMap as Map;
    use std::vec::Vec;

    type Nd<'a> = &'a str;
    type Ed<'a> = (Nd<'a>, Nd<'a>, &'a str, &'a str);

    impl<'a> dot::Labeller<'a, Nd<'a>, Ed<'a>> for Graph {
        fn graph_id(&'a self) -> dot::Id<'a> {
            dot::Id::new("rgraph").unwrap()
        }
        fn node_id(&'a self, n: &Nd) -> dot::Id<'a> {
            dot::Id::new(format!("{}", n)).unwrap()
        }
        fn edge_label<'b>(&'b self, edge: &Ed) -> dot::LabelText<'b> {
            let &(_, _, from, to) = edge;
            dot::LabelText::LabelStr(format!("{} -> {}", to, from).into())
        }
    }

    impl<'a> dot::GraphWalk<'a, Nd<'a>, Ed<'a>> for Graph {
        fn nodes(&'a self) -> dot::Nodes<'a, Nd<'a>> {
            self.iter().map(|s| s.0.as_str()).collect()
        }

        fn edges(&'a self) -> dot::Edges<'a, Ed<'a>> {

            let mut ins: Map<&'a str, &'a str> = Map::new();
            let mut out: Map<&'a str, &'a str> = Map::new();

            let mut nodes: Vec<&'a str> = Vec::new();

            for (name, node) in self.iter() {
                let name_str = name.as_str();
                nodes.push(name_str);
                for input in node.get_ins() {
                    ins.insert(input.as_str(), name_str);
                }
                for output in node.get_outs() {
                    out.insert(output.as_str(), name_str);
                }
            }
            self.bindings
                .iter()
                .map(|b| {
                         (*out.get(b.1.as_str()).expect("malformed graph"),
                          *ins.get(b.0.as_str()).expect("malformed graph"),
                          b.0.as_str(),
                          b.1.as_str())
                     })
                .collect()
        }

        fn source(&self, e: &Ed<'a>) -> Nd<'a> {
            let &(s, _, _, _) = e;
            s
        }

        fn target(&self, e: &Ed<'a>) -> Nd<'a> {
            let &(_, t, _, _) = e;
            t
        }
    }

}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

#[cfg(test)]
mod tests {

    #![macro_use]
    use super::*;

    fn get_test_graph() -> Graph {
        let mut g = Graph::new();
        g.add_node(create_node!(sink_1 ( input : u32) -> ()
                                 {
                                     println!("sink 1 {}", input);
                                 })).unwrap();

        g.add_node(create_node!(sink_2 ( name : u32) -> ()
                                 {
                                     println!("sink 2 {}", name);
                                 })).unwrap();

        g.add_node(create_node!(no_input () -> ( i : u32)
                                 {
                                     i =  1234;
                                     println!("produce {}", i);
                                 })).unwrap();
        g
    }

    #[test]
    fn info() {
        let g = get_test_graph();
        print_info(&g);
    }

    #[test]
    fn dot() {
        let g = get_test_graph();
        let mut output = Vec::new();
        dot::render(&g, &mut output).expect("it should draw");
        let dot_text = String::from_utf8(output).unwrap();
        println!("{}", dot_text);
    }

    #[test]
    fn dot2() {
        let mut g = get_test_graph();

        g.bind_asset("no_input::i", "sink_1::input")
            .expect("binding should exist");
        g.bind_asset("no_input::i", "sink_2::name")
            .expect("binding should exist");

        let mut output = Vec::new();
        dot::render(&g, &mut output).expect("it should draw");
        let dot_text = String::from_utf8(output).unwrap();
        println!("{}", dot_text);
    }
}
