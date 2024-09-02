import sub.*;

public class Main{
    public static final int answer = 42;
    private static Pair<T, X> pair;

	public static int main(String[] args){
		//System.out.println("Hello, World!");
		//java.lang.Class integer = int.class;
		//Car car = new Car();
		//int distance = car.drive();
		//car.init_thing(1, 7);
		/*boolean equal = new Object().equals(new Object());
		Empty empty = new Empty(5);
		int res = empty.add(answer);
		int x = answer * (empty.getNumber() - 1);
		return x;*/

		pair = new Pair(new T(), new X());

		return 0;
	}

	static class T{}
	static class X{}

	double test(int v1, int v2, int[][] v3, float v4, double v5, char v6, Object v7, int v8){
	    return 3;
	}

	public static class Inner{
	    public void cmonDoSomething(){}
	}

	public static class Pair<K, V>{
	    private K key;
	    private V value;

	    public Pair(K key, V value){
	        this.key = key;
	        this.value = value;
	    }
	}
}

