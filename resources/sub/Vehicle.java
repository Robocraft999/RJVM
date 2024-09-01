package sub;

public class Vehicle{
    private float modifier;
    protected char letter;

    public Vehicle(){
        this.modifier = 1.0f;
        this.letter = 'X';
    }

    public int drive(){
        return 42;
    }

    public void init_thing(int a, int b){
        int x = drive() + 12 + (a - b);
    }
}