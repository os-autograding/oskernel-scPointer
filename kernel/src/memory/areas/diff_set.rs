
use super::VmArea;

pub enum DiffSet {
        Unchanged,
        Removed,
        Shrinked,
        Splitted(VmArea, VmArea),
}