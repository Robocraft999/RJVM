import java.util.ArrayList;
import java.util.Iterator;

class Pair<K extends Comparable<K>, V> {
	// two generic types, K and V
	//
	// Both generic types are comparable two other variables of their own type
	
	public K key;
	// key of type K
	public V value;
	// value of type V

	public Pair(K key, V value) {
		this.key   = key;
		this.value = value;
	}
}

class Slot
<K extends Comparable<K>, V>
implements Iterable<Pair<K, V>>
{
	/* The class Slot has two generic types: K and V
	 * Both types extend Comparable<S> and are therefore comparable to other types of their own type
	 *
	 * The class Slot implements Iterable<Pair<K, V>>, this means you can iterate over it in a for-loop
	 *
	 *
	 * One slot corresponds to one index within the HashMap
	 * It stores all key-value pairs that are computed to bear the corresponding index
	 */
	public static int initialCapacity = 1;
	// How many elements should it be able to hold when created?
	// I should have implemented is as a linked-list, but it's too late now, get over it
	// Initialcapacity is set to 1 to decrease the memory consumption

	private ArrayList<Pair<K, V>> members;
	// list of the key-value pairs that are in this slot

	public Slot() {
		this(Slot.initialCapacity);
		// pass on with default values
	}

	public Slot(int intialCapacity) {
		this.members = new ArrayList<Pair<K, V>>(initialCapacity);
		// create ArrayList that holds all the members (key-value-pairs) of this slot
	}

	public boolean addPair(K key, V value) {
		// Add a Key-value pair to this slot
		if (this.containsKey(key)) {
			return false; // Key is already in this slot? can't add it again then
		}

		Pair<K, V> member = new Pair<K, V>(key, value); // Create a Pair instance to hold the values
		this.members.add(member); // Append to Arraylist

		return true; // done
	}

	public boolean containsKey(K key) {
		// check whether a key is contained in this slot

		for (Pair<K, V> member : this.members) {
			// for each member in the member-arraylist
			if (member.key.compareTo(key) == 0) {
				// use compareTo to check for key-equality
				return true;
			}
		}
		return false;
	}

	public Pair<K, V> getPair(K key) throws IllegalArgumentException {
		// Retrieve a key-value pair to a containing a given key

		for (Pair<K, V> member : this.members) {
			// for each member in the member-arraylist
			if (member.key.compareTo(key) == 0) {
				// check for key-equality with compareTo-function
				// compareTo function is available on this type, because it extends comparable
				return member;
			}
		}
		throw new IllegalArgumentException("Key was not found in Slot.");
	}

	public Iterator<Pair<K, V>> iterator() {
		// return an iterator over the arraylist
		return this.members.iterator();
	}

	public boolean set(K key, V value) throws IllegalArgumentException {
		// change the value associated with the key

		Pair<K, V> pair = this.getPair(key);
		// retrieve the pair

		pair.value = value; // Overwrite value of Pair
		return true; // return true because the value was changed
	}
	
	public boolean remove(K key) {
		// search for the member-pair with a matching key
		int i;
		for (i = 0; i != this.members.size(); i++) {
			Pair<K, V> member = this.members.get(i); 
			
			// are the keys equal?
			if (member.key.compareTo(key) == 0) {
				// exit the loop!
				break;
			}
		}
		
		boolean removed = false;
		
		// if the member was found, remove it
		if (i != this.members.size()) {
			this.members.remove(i);
			removed = true;
		}
		
		// return whether the removal was successfull
		return removed;
	}
}

public class HashMap<K extends Comparable<K>, V> {
	/* K extends Comparable<K> means, that you can use the compareTo method on variables of type K
	 * same goes for V
	 * We have two dynamic types here that are comparable to their own type respectively
	 */
	public static double defaultLoad     = 0.66; 
	// How many of the slots from your HashMap should be used before new ones are allocated
	public static int    defaultCapacity = 128;
	// What is the standard capacity, used in the constructor

	private double count;
	// How many elements are there in this HashMap
	private ArrayList<Slot<K, V>> slots;
	// The slots used to store values
	private double maxLoadFactor;
	// what is the load factor for this HashMap

	public HashMap() {
		this(HashMap.defaultLoad, HashMap.defaultCapacity);
		// No Arguments passed, construct with default values
	}

	public HashMap(double maxLoadFactor, int initialCapacity) {
		this.count         = 0;
		// Zero elements to start with
		this.maxLoadFactor = maxLoadFactor;
		// Set local loadFactor for this HashMap only
		this.slots         = new ArrayList<Slot<K, V>>(initialCapacity);
		// I'm using an ArrayList here because it supplies me with easy access methods, I dislike Java Arrays.
		// Could just as well been an Array

		for (int i = 0; i < initialCapacity; i++) {
			this.slots.add(new Slot<K, V>());
		}
		// Fill the slot List with empty slots, because there is no data yet

	}

	public boolean add(K key, V value) {
		// return: 
		// 	true if added successfully
		// 	false if key already used
		
		Slot<K, V> slot = this.getSlot(key);
		// What slot does this key compute into
		
		boolean success = slot.addPair(key, value);
		// store success value to perform operations dependant on it

		if (success) {
			this.count += 1;
			// There is now one more element in this HashMap
			this.expandIfNecessary();
			// Checks whether the load factor is exceeded and expands the slots if necessary
		}

		return success; // return whether we were successfull
	}

	public boolean addPair(Pair<K, V> pair) {
		// Just a function to allow passing a Pair
		return this.add(pair.key, pair.value);
	}

	public double elementCount() {
		// return how many elements there are in this Map
		return this.count;
	}

	public boolean expandIfNecessary()
	{
		if (this.loadFactor() > this.maxLoadFactor) {
			// Are there more elements in this map than we can efficiently support?
			return this.resizeSlotList(this.slots.size() * 2);
			// yes? then resize the slots to double it's current capacity
		}
		return false;
	}

	public V get(K key) throws IllegalArgumentException {
		// Retrieve the value corresponding to a given Key
		Slot<K, V> slot = this.getSlot(key);
		// Slot which the key is hashed into

		if (slot.containsKey(key)) {
			// Key is in slot?
			return slot.getPair(key).value;
			// yes? return the value
		}
		throw new IllegalArgumentException("Invalid key: Key not found in HashMap.");
		// key was not found within the slot, so it's not in the HashMap, throw an Exception
	}

	private Slot<K, V> getSlot(K key) {
		// returns the slot in which a key should be stored
		
		int        index = key.hashCode() % this.slots.size();
		// Hash the key and then modulo the slot capacity to ensure we're operating within bounds
		Slot<K, V> slot  = this.slots.get(index);
		return slot;
	}

	public double loadFactor() {
		return this.count / (double) (this.slots.size());
		// The load factor is computed by dividing the element count by the capacity
		// To ensure efficiency, I suggest that only 66% of a HashMaps capacity is used
		// That way, Hash collisions can be avoided
	}

	public boolean resizeSlotList(int newCapacity) {
		ArrayList<Slot<K, V>> oldSlots = this.slots;
		// Store slots before resize, we will need them to iterate over all elements within them

		ArrayList<Slot<K, V>> newSlots = new ArrayList<Slot<K, V>>(newCapacity);
		// New slot ArrayList with increased capacity

		this.slots = newSlots;
		// Set slots to class member

		for (int i = 0; i < newCapacity; i++) {
			newSlots.add(new Slot<K, V>());
		}
		// fill arrayList with empty Slots because there is no data yet

		oldSlots.stream().forEach(
			// For each slot in the old slot list
			slot -> slot.forEach(
				// for each element within this old slot
				member -> this.addPair(member)
				// Add the element to the newly acquired slot list
			)
		);
		// Why is it necessary to make a new arraylist and not just expand the old list, you might ask
		/* Because the index of a key depends on what the capacity of our list is.
		 * 
		 * If I was to insert a key with a hashCode of 1011 to an index of 1 into the HashMap whilst the capacity is 10, this would make perfect sense
		 * Then I expand the list containing the slots to a capacity of 20
		 * Now I want to retrieve the value of the key inserted, so I compute the hash again
		 * Hash is 1011, Modulo 20, we get 11 as the index
		 * But now, we can't find him in slot 11 anymore because we just resized the list and not remapped the elements
		 * Thank you for your attention
		 */

		return true; // Success
	}

	public boolean remove(K key) {
		// Remove a key-value-pair from the HashMap
		
		Slot<K, V> slot = this.getSlot(key);
		// Get the Slot corresponding the keys index
		return slot.remove(key); // Tell the slot to remove the pair
		// return the value the slot gave us, because this is our success value
	}

	public boolean set(K key, V value) throws IllegalArgumentException {
		// Returns whether the collection has changed
		// Set the value of a key to a new value
		
		Slot<K, V> slot = this.getSlot(key);
		// Which slot is the pair in?
		return slot.set(key, value); // tell the slot to do it
	}

	public boolean setPair(Pair<K, V> pair) throws IllegalArgumentException {
		// Returns whether the collection has changed
		
		// see set(), just a pass-on function
		return this.set(pair.key, pair.value);
	}
}
