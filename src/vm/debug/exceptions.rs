use std::cell::RefCell;
use log::error;

pub struct ExceptionHelper{
    history: RefCell<Vec<String>>
}

impl ExceptionHelper{
    pub fn new() -> Self{
        Self{
            history: RefCell::new(Vec::new())
        }
    }
    
    pub fn push(&self, line: String){
        self.history.borrow_mut().push(line);
    }
    
    pub fn print(&self){
        for line in self.history.borrow().iter(){
            error!("{}", line);
        }
    }
}