package sub;

public class Vehicle{
    private float modifier;
    protected char letter;

    public Vehicle(){
        this.modifier = 1.0f;
        this.letter = 'X';
    }

    public Vehicle(char letter){
        this.modifier = 1.0f;
        this.letter = letter;
    }

    public int drive(){
        return 42;
    }

    public void init_thing(int a, int b){
        int x = drive() + 12 + (a - b);
    }
}