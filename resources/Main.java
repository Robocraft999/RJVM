//import sub.*;
import java.lang.RuntimeException;
import java.util.Properties;
//import sun.misc.VM;

import java.security.AccessController;
import java.security.PrivilegedAction;
import sun.security.action.GetPropertyAction;

public class Main{
    public static final int answer = 42;

	public static void main(String[] args){
		//String var1 = (String)AccessController.doPrivileged(new GetPropertyAction("java.awt.graphicsenv", (String)null));
		String var1 = System.getenv("java.awt.graphicsenv");
		System.out.println(var1);
		//Y context = Y.getContext();

        //Car car = new Car();
	}

	/*static class X{
	    public X(){
	        System.out.println("X before");
	        Y.getContext();
	        System.out.println("X after");
	    }
	}

	static class Y{
	    static Y mainContext;
	    private X x;
	    private static final Object lock = new Object();

	    private static void initMainContext(){

	    }

	    public static Y getContext(){
	        if (mainContext != null){
	            return mainContext;
	        }
	        Y context = AccessController.doPriviliged(new PriviligedAction<Y>(){
                public Y run(){
                    synchronized (lock){
                        if (mainContext == null){
							initMainContext();
						} 
                    }
                }
	        });
	    }

	    private Y(){
	        System.out.println("Y before");
	        this.x = new X();
	        System.out.println("Y after");
	    }

	}*/
}

