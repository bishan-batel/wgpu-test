use std::{
    cell::{BorrowMutError, RefCell},
    rc::{Rc, Weak},
};

use hashbrown::HashMap;

use crate::core::name::StringName;

#[derive(Debug)]
pub struct Node {
    name: StringName,

    children: Vec<Rc<RefCell<Node>>>,

    /// child name to map lookup
    children_name_lookup: HashMap<StringName, u32>,

    parent: Weak<RefCell<Node>>,

    self_ptr: Weak<RefCell<Node>>,
}

/// Naming error
#[derive(thiserror::Error, Debug)]
pub enum RenameError {
    #[error("Cannot name a node to the same name as a sibling (index: {sibling_index})")]
    DuplicateName { sibling_index: u32 },

    #[error("Cannot rename child while its parent has been borrowed elsewhere: {0}")]
    FailedToBorrowParent(#[from] std::cell::BorrowMutError),
}

impl Node {
    #[must_use]
    pub fn new() -> Rc<RefCell<Self>> {
        Rc::new_cyclic(|s| {
            RefCell::new(Self {
                name: "Node".into(),
                children: vec![],
                parent: Weak::new(),
                children_name_lookup: HashMap::new(),
                self_ptr: s.clone(),
            })
        })
    }

    #[must_use]
    pub fn name(&self) -> &StringName {
        &self.name
    }

    /// whether or not this node has a parent attached or not
    #[must_use]
    pub fn has_parent(&self) -> bool {
        self.parent.upgrade().is_some()
    }

    pub fn add_child(&mut self, child: Rc<RefCell<Node>>) -> eyre::Result<()> {
        let child_name = {
            let Ok(mut child_ref) = child.try_borrow_mut() else {
                eyre::bail!("Cannot borrow child as mutable.");
            };

            if child_ref.has_parent() {
                eyre::bail!("A child cannot have multiple parent nodes at once");
            }

            while let Some(_) = self.child_name_to_index(child_ref.name()) {
                child_ref.name = StringName::from(format!("{}_", &child_ref.name));
            }

            // update link in child
            child_ref.parent = self.self_ptr.clone();

            // get a copy of its name for book keeping
            child_ref.name().clone()
        };

        let idx = self.children.len();

        // push child to list
        self.children.push(child);

        // book keeping
        self.children_name_lookup.insert(child_name, idx as u32);

        Ok(())
    }

    /// Attempts to rename this node to the given name loosely, this may mangle the name if there
    /// are name conflicts with siblings. This returns the new name of this node.
    ///
    /// # Errors
    ///
    /// This function will return an error if it cannot borrow its parent mutably.
    pub fn rename(
        &mut self,
        new_name: impl Into<StringName>,
    ) -> Result<&StringName, BorrowMutError> {
        self.set_name(new_name.into(), true)
            .map_err(|err| match err {
                RenameError::DuplicateName { .. } => unreachable!("set_name should not fail if there are duplicate names and the 'mangle' parameter is set on"),
                RenameError::FailedToBorrowParent(err) => err,
            })
    }

    /// Attempts to set the name of this node to a specific value.
    ///
    /// # Errors
    ///
    /// This function will return an error if it cannot borrow its parent mutably or if the given
    /// name conflicts with the name of a sibling node.
    pub fn set_name(
        &mut self,
        new_name: StringName,
        mangle_name: bool,
    ) -> Result<&StringName, RenameError> {
        let Some(parent) = self.parent.upgrade() else {
            // if there is no parent then there is no restrictions
            self.name = new_name;
            return Ok(&self.name);
        };

        let mut parent = parent.try_borrow_mut()?;

        // fail if a sibling has the same name and mangle name is turned off
        if let Some(sibling_idx) = parent.children_name_lookup.get(&new_name)
            && !mangle_name
        {
            return Err(RenameError::DuplicateName {
                sibling_index: *sibling_idx,
            });
        }

        let new_name = {
            let mut name = new_name;

            // while a parent has a sibling of the name, just add '_' to it
            while let Some(_) = parent.child_name_to_index(&name) {
                name = StringName::from(format!("{}_", &self.name));
            }

            name
        };

        let child_index = parent.children_name_lookup[&self.name];

        // remove old name's lookup
        parent.children_name_lookup.remove(&self.name);

        parent
            .children_name_lookup
            .insert(new_name.clone(), child_index);

        self.name = new_name;

        Ok(&self.name)
    }

    pub fn iter_children(&self) -> std::slice::Iter<'_, Rc<RefCell<Node>>> {
        self.children.iter()
    }

    pub fn query<Q>(&self, buf: &mut Vec<Rc<RefCell<Node>>>, query: &Q)
    where
        Q: Fn(&Rc<RefCell<Node>>) -> bool,
    {
        for child in self.children.iter() {
            if query(child) {
                buf.push(child.clone());
            }

            if let Some(child) = child.try_borrow().ok() {
                child.query(buf, query);
            }
        }
    }

    /// How many children does this node have
    #[must_use]
    pub fn child_len(&self) -> usize {
        self.children.len()
    }

    #[must_use]
    pub fn nth_child(&self, idx: usize) -> Option<Rc<RefCell<Node>>> {
        Some(self.children.get(idx)?.clone())
    }

    #[must_use]
    pub fn child_name_to_index(&self, name: &StringName) -> Option<usize> {
        self.children_name_lookup.get(name).map(|i| *i as usize)
    }

    /// gets a child who has the given nmae
    #[must_use]
    pub fn get_child_by_name(&self, name: &StringName) -> Option<Rc<RefCell<Node>>> {
        Some(self.children[self.child_name_to_index(name)?].clone())
    }
}

#[cfg(test)]
mod test {
    use crate::{core::name::StringName, scene::node::Node};

    #[test]
    fn naming_no_parent() {
        let node = Node::new();

        let mut node = node.borrow_mut();

        assert_eq!(node.name(), "Node");

        node.set_name("Hello".into(), false).unwrap();

        assert_eq!(node.name(), "Hello");

        node.set_name("Node2".into(), false).unwrap();

        assert_eq!(node.name(), "Node2");

        node.rename(":)").unwrap();

        assert_eq!(node.name(), ":)");
    }

    #[test]
    fn simple_parent() {
        color_eyre::install().unwrap();

        let parent = Node::new();

        assert_eq!(parent.borrow().child_len(), 0);

        let child_name = StringName::from("Child");

        assert!(parent.borrow().get_child_by_name(&child_name).is_none());

        let child = Node::new();
        child.borrow_mut().rename(child_name.clone()).unwrap();

        parent.borrow_mut().add_child(child.clone()).unwrap();

        assert!(child.borrow().has_parent());

        assert_eq!(parent.borrow().child_len(), 1);
        assert_eq!(child.borrow().name(), &child_name);

        {
            // get the child, rename it, check the re ference really does match
            let node = parent.borrow().get_child_by_name(&child_name).unwrap();

            let new_name = StringName::from("Huh");

            {
                let first_child = parent.borrow().nth_child(0).unwrap();

                first_child.borrow_mut().rename(new_name.clone()).unwrap();
            }

            assert_eq!(&new_name, node.borrow().name());
        }
    }
}
