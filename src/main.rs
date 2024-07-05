pub mod blob;
pub mod dense;
pub mod table;

fn main() {}

trait Node: 'static {
    type Input;
    type Output;

    fn execute(input: &Self::Input) -> Option<Self::Output>;
}
