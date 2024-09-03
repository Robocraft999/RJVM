package sub;

public class Car extends Vehicle{

    private Vehicle vehic = this;

    public Car(){
        super();
        this.letter = 'C';
    }

    public int drive(){
        return 69;
    }
}