package sub;

public class Car extends Vehicle{

    private Vehicle vehic = this;

    public Car(){
        super('C');
    }

    public int drive(){
        return 69;
    }
}