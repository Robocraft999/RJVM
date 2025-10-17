use log::error;

pub struct ExceptionHelper{
    history: Vec<String>
}

impl ExceptionHelper{
    pub fn new() -> Self{
        Self{
            history: Vec::new()
        }
    }
    
    pub fn push(&mut self, line: String){
        self.history.push(line);
    }
    
    pub fn print(&self){
        for line in self.history.iter(){
            error!("{}", line);
        }
    }
}