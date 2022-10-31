use std::collections::{BTreeMap, HashMap};
use serde::{Deserialize, Serialize};

// Json representation of the data struct
// Map Fragment name : List<List <Fragment Names>>
#[derive(Serialize, Deserialize, Debug, Default)]
struct Grammar(HashMap<String, Vec<Vec<String>>>);

#[derive(Clone, Debug, Copy)]
struct FragmentId(usize);

#[derive(Clone, Debug)]
enum Fragment {
    // nonterminal contains a vector of fragments (some might be non-terminal)
    NonTerminal(Vec<FragmentId>),
    // Ordered list of fragments
    Expression(Vec<FragmentId>),
    // terminal results to bytes
    Terminal(Vec<u8>),
}

// Rust representation: transformed into nested structure
#[derive(Debug, Default)]
struct GrammarRust {
    // all types
    fragments: Vec<Fragment>,
    // Mapping of non-terminal names to fragment identifiers
    name_to_fragment: BTreeMap<String, FragmentId>,
}

// turns json representation into rust data structure
impl GrammarRust {
    fn new(grammar: &Grammar) -> Self {
        // create new grammar structure
        let mut ret = GrammarRust::default();

        // parse the input grammar to create non-term fragment names
        for (non_term, _) in grammar.0.iter() {
            // have not seen the fragment before?
            assert!(!ret.name_to_fragment.contains_key(non_term),
                    "Duplicate non-terminal definition, fail");

            // allocate a new empty fragment
            let fragment_id = ret.allocate_fragment(Fragment::NonTerminal(Vec::new()));

            // add name resolution to the fragment
            ret.name_to_fragment.insert(non_term.clone(), fragment_id);
        }

        // having all non-term names, allocate their term/non-term extensions
        for (non_term, fragments) in grammar.0.iter() {
            print!("Handling {:?}\n", non_term);
            // get the non-terminal fragment identifier
            let fragment_id = ret.name_to_fragment[non_term];
            print!("Its id: {:?}\n", fragment_id);

            // Expressions
            let mut expressions = Vec::new();

            // go through all sub-fragments (vectors of fragment names)
            for js_sub_fragment in fragments {
                // Options for this sub fragment
                let mut options = Vec::new();

                for option in js_sub_fragment {
                    print!("Option: {:?}\n", option);
                    // if option is one of the previously found non-terminals
                    let fragment_id = if let Some(&non_terminal) =
                    ret.name_to_fragment.get(option) {
                        ret.allocate_fragment(
                            Fragment::NonTerminal(vec![non_terminal]))
                    } else {
                        // Convert the terminal bytes into a vector
                        // and create a new fragment containing it
                        ret.allocate_fragment(
                            Fragment::Terminal(
                                option.as_bytes().to_vec()))
                    };
                    options.push(fragment_id);
                }
                // Allocate a new fragment for all the options
                // List of Options - Vec<String>
                expressions.push(
                    ret.allocate_fragment(Fragment::Expression(options)));
            }

            // get access to the fragment we want to change
            let fragment = ret.lookup_fragment_mut(fragment_id);

            // Overwrite the terminal definition
            // expressions - Vec<Vec<String>>
            *fragment = Fragment::NonTerminal(expressions);
        }

        // print!("{:#?}\n", ret);
        ret
    }

    pub fn allocate_fragment(&mut self, fragment: Fragment) -> FragmentId {
        // get a unique fragment ID
        let fragment_id = FragmentId(self.fragments.len());

        // store the fragment
        self.fragments.push(fragment);

        fragment_id
    }

    #[inline]
    pub fn lookup_fragment_mut(&mut self, id: FragmentId) -> &mut Fragment {
        &mut self.fragments[id.0]
    }
}

fn main() -> std::io::Result<()> {
    // serialize grammar input
    let grammar: Grammar = serde_json::from_slice(&std::fs::read("grammar.json")?)?;

    let gram = GrammarRust::new(&grammar);
    print!("{:#?}\n", gram);

    Ok(())
}
