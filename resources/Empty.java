public class Empty{
    public static double double_edged_sword = 13.5f;
    protected static final char nl = '\n';
    private int number;

    public Empty(int number){
        this.number = number;
    }

    public int add(int other){
        this.number += other + (int)double_edged_sword;
        return this.number;
    }

    public int getNumber(){
        return this.number;
    }
}