use pyo3::prelude::*;

use crate::node::Node;

#[pyclass]
pub struct SLPDEX {
    node: Node,
}

#[pymethods]
impl SLPDEX {
    #[staticmethod]
    fn connect(addr: &str) -> PyResult<SLPDEX> {
        let mut node = Node::new();
        node.connect(addr)?;
        Ok(SLPDEX {
            node
        })
    }

    fn run_forever(&mut self) -> PyResult<()> {
        self.node.run_forever();
        Ok(())
    }
}
