fn main() {
    println!("Hello, world!");



    // Sharing Versus Mutation

    // Rust ensures no ref will ever point to a variable that has gone out of scope. But there are other ways to introduce dangling pointers. Example:
    let v = vec![4, 8, 19, 27, 34, 10];
    let r = &v;
    let aside = v; // move vector to aside
    r[0]; // bad: uses 'v', which is now uninitialized

    // The assignment to aside moves the vector, leaving v uninitialized, turning r into a dangling pointer (see page 185 for diagram).

    // Although v stays in scope for r's entire lifetime, the problem here is that v's value gets moved elsewhere, leaving v uninitialized while r still refers to it. Naturally, Rust catches the error:
    // cannot move out of 'v' because it is borrowed

    // Throughout its lifetime, a shared ref makes its referent read-only. We may not assign to the referent or move its value elsewhere. In this code, r's lifetime contains the attempt to move the vector, so Rust rejects the program. With changes, we can fix it:
    let v = vec![4, 8, 19, 27, 34, 10];
    {
        let r = &v;
        r[0]; // ok, vector is still there
    }
    let aside = v;

    // In this version, r goes out of scope earlier, the refs lifetime ends before v is moved aside, and all is well.

    // Another example that wreaks havoc. Suppose we have a handy function to extend a vector with the elements of a slice:
    fn extend(vec: &mut Vec<f64>, slice: &[f64]) {
        for elt in slice {
            vec.push(*elt);
        }
    }

    // This is a less flexible (and much less optimized) version of the standard library's extend_from_slice method on vectors. We can use it to build up a vector from slices of other vectors or arrays:
    let mut wave = Vec::new();
    let head = vec![0.0, 1.0];
    let tail = [0.0, -1.0];

    extend(&mut wave, &head); // extend wave with another vector
    extend(&mut wave, &tail); // extend wave with an array

    assert_eq!(wave, vec![0.0, 1.0, 0.0, -1.0]);

    // We've built up one period of a sine wave here. If we want to add another undulation, can we append the vector to itself?
    extend(&mut wave, &wave);
    assert_eq!(wave, vec![0.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0, -1.0]);

    // This may look fine on casual inspection. But remember that when we add an element to a vector, if its buffer is full, it must allocate a new buffer with more space. Suppose wave starts with space for four elements, and so must allocate a larger buffer when extend tries to add a fifth. See page 187 for diagram illustrating the change.

    // The extend function's vec argument borrows wave (owned by the caller), which has allocated itself a new buffer with space for eight elements. But slice continues to point to the old four-element buffer which has been dropped. 

    // This sort of problem isn't unique to Rust. Modifying collections while pointing into them is delicate territory in many languages (like C++ or Java). What's especially difficult about this sort of bug is that is doesn't happen all the time. In testing, our vector might always happen to have enough space, the buffer might never be reallocated, and the problem might never come to light.

    // Rust, however, reports the problem with our call to extend at compile time:
    // Cannot borrow `wave` as immutable because it is also borrowed as mutable...

    // In other words, we may borrow a mutable ref to the vector, and we may borrow a shared ref to its elements, but those two refs lifetimes may not overlap. In our case, both refs lifetimes contain the call to extend so Rust rejects the code.

    // These errors both stem from violations of Rust's rules for mutation and sharing:
    // 1. Shared access is read-only access. Values borrowed by shared refs are read-only. Across the lifetime of shared ref, neither its referent, nor anything reachable from that referent, can be changed by anything. There exist no live mutable refs to anything in that structure. Its owner is held read-only, and so on. It's frozen.
    // 2. Mutable access is exclusive access. A value borrowed by a mutable ref is reachable exclusively via that ref. Across the lifetime of a mutable ref, there is no other usable path to its referent, or to any value reachable from there. The only refs whose lifetimes may overlap with a mutable ref are those we borrow from the mutable ref itself.

    // Rust reported the extend example as a violation of the second rule. Since we've borrowed a mutable ref to wave, that mutable ref must be the only way to reach the vector or its elements. The shared ref to the slice is itself another way to reach the elements, violating the second rule.

    // Rust could also treat our bug as a violation of the first rule. Since we've borrowed a shared ref to wave's elements, the elements and the Vec itself are all read-only. We can't borrow a mutable ref to a read-only value. See page 189 for diagram showing the diffs on refs.

    // In both cases, the path of ownership leading to the referent cannot be changed for the refs lifetime. For a shared borrow, the path is read-only. For a mutable borrow, it's completely inaccessible. So there's no way for the program to do anything that will invalidate the ref.

    // Pairing the principles down to the simplest possible examples:
    let mut x = 10;
    let r1 = &x;
    let r2 = &x; // ok, multiple shared borrows permitted
    x += 10; // error, cannot assign to 'x' because it is borrowed
    let m = &mut x; // error, cannot borrow 'x' as mutable because it is also borrowed as immutable.

    let mut y = 20;
    let m1 = &mut y;
    let m2 = &mut y; // error, cannot borrow as mutable more than once
    let z = y; // error, cannot use 'y' because it was mutably borrowed

    // It is OK to re-borrow a shared ref from a shared ref:
    let mut w = (107, 109);
    let r = &w;
    let r0 = &r.0; // ok, re-borrowing shared as shared
    let m1 = &mut r.1; // error, can't re-borrow shared as mutable

    // We can re-borrow from a mutable reference:
    let mut v = (136, 139);
    let m = &mut v;
    let m0 = &mut m.0; // ok, re-borrowing mutable from mutable
    *m = 137;
    let r1 = &m.1; // ok, re-borrowing shared from mutable, and doesn't overlap with m0
    v.1; // error, access through other paths still forbidden

    // These restrictions are pretty tight. Turning back to our attempted call extend(&mut wave, &wave), there's no quick and easy way to fix up the code to work the way we'd like. Rust applies these rules everywhere. If we borrow, say, a shared ref to a key in a HashMap, we can't borrow a mutable ref to the HashMap until the shared refs lifetime ends.

    // But there's good justification for this. Designing collections to support unrestricted, simultaneous iteration and modification is difficult, and often precludes simpler, more efficient implementations. See page 191 for how other languages do or don't do this.

    // Another example of the kind of bug these rules catch. Consider the following C++ code, meant to manage a file descriptor. To keep things simple, we're only going to show a constructor and copying assignment operator, and we're going to omit error handling:
    struct File {
        int desciptor;

        File(int d) : descriptor(d) {}

        File& operator=(const File &rhs) {
            close(descriptor);
            descriptor = dup(rhs.descriptor);
        }
    };

    // The assignment operator is simple enough, but fails badly in a situation life this:
    File f(open("foo.txt", ...));
    ...
    f = f;

    // If we assign a FIle to itself, both rhs, and *this are the same object, so operator= closes the very file descriptor it's about to pass to dup. We destroy the same resource we were meant to copy.

    // In Rust, the analogous code would be:
    struct File {
        descriptor: i32
    }

    fn new_file(d: i32) -> File {
        File { descriptor: d }
    }

    fn clone_from(this: &mut File, rhs: &File) {
        close(this.descriptor);
        this.descriptor = dup(rhs.descriptor);
    }

    // Aside: The above isn't idiomatic Rust. There are excellent ways to give Rust types their own constructor functions and methods, which are covered in chapt 9. The above use is for example purposes.

    // If we write the Rust code corresponding to the use of FIle, we get:
    let mut f = new_file(open("foo.txt", ...));
    ...
    clone_from(&mut f, &f);

    // Rust, of course, refuses to compile the code:
    // cannot borrow `f` as immutable because it is also borrowed as mutable..

    // This should look familiar. It turns out that two classic C++ bugs, failure to cope with self-assignment, and using invalidated iterators are the same underlying kind of bug. In both cases, code assumes it's modifying one value while consulting another, when in fact they're both the same value. By requiring mutable access to be exclusive, Rust has fended off a wide class of everyday mistakes.

    // The immiscibility of shared and mutable refs really demonstrates its value when writing concurrent code. A data race is possible only when some value is both mutable and shared between threads, which is exactly what Rust's reference rules eliminate. A concurrent Rust program that avoids unsafe code is free of data races by construction (covered in Chapter 19). In summary, concurrency is much easier to use in Rust than in most other languages.





}
