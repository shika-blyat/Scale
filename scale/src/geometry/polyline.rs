use super::Vec2;
use cgmath::InnerSpace;
use serde::{Deserialize, Serialize};
use std::ops::Index;
use std::slice::{Iter, IterMut};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PolyLine(Vec<Vec2>);

impl From<Vec<Vec2>> for PolyLine {
    fn from(x: Vec<Vec2>) -> Self {
        Self(x)
    }
}

impl PolyLine {
    pub fn new(x: Vec<Vec2>) -> Self {
        Self(x)
    }
    pub fn with_capacity(c: usize) -> Self {
        Self(Vec::with_capacity(c))
    }

    pub fn length(&self) -> f32 {
        self.0.windows(2).map(|x| (x[1] - x[0]).magnitude()).sum()
    }

    pub fn n_points(&self) -> usize {
        self.0.len()
    }

    pub fn extend<'a>(&mut self, s: impl IntoIterator<Item = &'a Vec2>) {
        self.0.extend(s)
    }

    pub fn get(&self, id: usize) -> Option<&Vec2> {
        self.0.get(id)
    }

    pub fn pop(&mut self) -> Option<Vec2> {
        self.0.pop()
    }

    pub fn push(&mut self, item: Vec2) {
        self.0.push(item)
    }

    pub fn pop_first(&mut self) -> Option<Vec2> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.remove(0))
        }
    }

    pub fn last(&self) -> Option<&Vec2> {
        self.0.last()
    }

    pub fn first(&self) -> Option<&Vec2> {
        self.0.first()
    }

    pub fn last_mut(&mut self) -> Option<&mut Vec2> {
        self.0.last_mut()
    }

    pub fn first_mut(&mut self) -> Option<&mut Vec2> {
        self.0.first_mut()
    }

    pub fn as_slice(&self) -> &[Vec2] {
        self.0.as_slice()
    }

    pub fn clear(&mut self) {
        self.0.clear()
    }

    pub fn iter(&self) -> Iter<Vec2> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<Vec2> {
        self.0.iter_mut()
    }
}

impl Index<usize> for PolyLine {
    type Output = Vec2;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}
