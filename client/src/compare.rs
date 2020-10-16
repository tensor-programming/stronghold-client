use std::hash::Hash;

pub trait CompareId {
    type ID: Hash + PartialEq + Eq;

    fn id(&self) -> &Self::ID;
}

#[derive(Hash, PartialEq, Eq, Debug, Clone, PartialOrd, Ord, Default)]
pub struct IdCompare<T: CompareId>(pub T);

impl<T> IdCompare<T>
where
    T: CompareId,
{
    pub fn new(id: T) -> Self {
        Self(id)
    }
}

impl<T> CompareId for IdCompare<T>
where
    T: CompareId,
{
    type ID = T::ID;

    fn id(&self) -> &Self::ID {
        self.0.id()
    }
}
