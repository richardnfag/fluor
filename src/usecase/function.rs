use crate::entity::function::Function;
use crate::adapter::function::RepositoryInterface;

pub trait UseCaseInterface {
    fn new(repository: RepositoryInterface) -> Result<Function, String>;
    fn search() -> Self;
    fn find_all() -> Vec<Function>;
    fn store() -> Self;
    fn delete() -> Result<String, String>;
}