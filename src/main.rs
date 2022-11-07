use std::cell::Cell;
use std::collections::{BTreeMap, HashMap};
use std::time::Instant;
use serde::{Deserialize, Serialize};
use rand::Rng;

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

    // Cached fragment identifier for the start node
    start: Option<FragmentId>,

    // Mapping of non-terminal names to fragment identifiers
    name_to_fragment: BTreeMap<String, FragmentId>,

    // Xorshift seed
    // in cell so that we do not need mutable access
    // https://doc.rust-lang.org/std/cell/
    seed: Cell<usize>
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
            // get the non-terminal fragment identifier
            let fragment_id = ret.name_to_fragment[non_term];

            // Expressions
            let mut expressions = Vec::new();

            // go through all sub-fragments (vectors of fragment names)
            for js_sub_fragment in fragments {
                // Options for this sub fragment
                let mut options = Vec::new();

                for option in js_sub_fragment {
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

        // Resolve the start node
        ret.start = Some(ret.name_to_fragment["<start>"]);

        // print!("{:#?}\n", ret);
        ret
    }
    // Initialize the RNG
    pub fn seed(&self, val: usize){
        self.seed.set(val);
    }

    // get a random value
    pub fn rand(&self) -> usize{
        let mut seed = self.seed.get();
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 43;

        self.seed.set(seed);
        seed
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

    #[inline]
    pub fn lookup_fragment(&self, id: FragmentId) -> &Fragment {
        &self.fragments[id.0]
    }

    #[inline]
    pub fn lookup_fragment_nonterm(&self, id: FragmentId) -> &[FragmentId] {
        // Match control flow action (?)
        if let Fragment::NonTerminal(x) = &self.fragments[id.0]{
            x
        }else{
            panic!("Was not a non-terminal!");
        }
    }

    pub fn generate(&self, stack: &mut Vec<FragmentId>, buf: &mut Vec<u8>) {
        // get access to the start node
        let start = self.start.unwrap();

        // start off working on start
        stack.clear();
        stack.push(start);

        while !stack.is_empty() {
            // unwrap makes sure the option is not a None
            let cur = stack.pop().unwrap();

            match self.lookup_fragment(cur) {
                Fragment ::NonTerminal(options) => {
                    let sel = options[self.rand() % options.len()];
                    stack.push(sel);
                    // print!("Non-terminal: {:?}\n", sel);
                }
                Fragment::Expression(expr) => {
                    // we must process all of these in sequence
                    // take expr slice and append all elements to stack vec
                    expr.iter().rev().for_each(|x| stack.push(*x));
                }
                Fragment::Terminal(value) => {
                    buf.extend_from_slice(value);
                    // print!("TERM\n");
                    if buf.len() > 1024*1024 {
                        break;
                    }
                }
            }
            // let _ = stack.pop();
        }

    }
}

fn main() -> std::io::Result<()> {
    // serialize grammar input
    let grammar: Grammar = serde_json::from_slice(&std::fs::read("test.json")?)?;
    let gram = GrammarRust::new(&grammar);
    let mut rng = rand::thread_rng();
    gram.seed(rng.gen::<i32>() as usize);
    // print!("{:#?}\n", gram);

    let mut buf = Vec::new();
    let mut stack = Vec::new();
    let mut generated = 0usize;
    let mut it = Instant::now();

    for iters in 1u64.. {
        buf.clear();
        gram.generate(&mut stack, &mut buf);
        generated += buf.len();

        if (iters & 0xffff) == 0{
            let elapsed = (Instant::now() - it).as_secs_f64();
            let bytes_per_sec = generated as f64 / elapsed;
            print!("Bytes per sec: {:12.0} | Example: {:#?}\n", bytes_per_sec, String::from_utf8_lossy(&buf));
        }
    }
    Ok(())
}
