### Hypercubes: a low hanging fruit
Counting the lattice points in a hypercube is possible in linear time proportional to the number of dimensions. Consider the 2D case:

`Graphics[{Red, Rectangle[{-5, -5}, {5, 5}], Black, Point /@ Tuples[Range[-6, 6], 2]}]`
![Square graphed with lattice points](/files/squarelatticeeasy.svg)

The square has a side length of 10 and is centered at the origin. Because each of the vertices corresponds with a lattice point, the contained lattice points are a 2-dimensional range from the lower left corner (x&#8321;,y&#8321;) to the upper right corner (x&#8322;,y&#8322;): {(x,y) | x&#8714; \[x&#8321;,x&#8322;\], y&#8714; \[y&#8321;,y&#8322;\]}. The total point count is (x&#8322;-x&#8321;)(y&#8322;-y&#8322;). For this case, the number of lattice points is thus (5-(-5))(5-(-5))=100.

How about when the square's vertices are not lattice points?

`Graphics[{Red, Rectangle[{-5.5, -5.5}, {5.5, 5.5}], Black, Point /@ Tuples[Range[-6, 6], 2]}]`
![Square graphed with lattice points](/files/squarelattice.svg)

 This can be reduced this to the original case by taking the floor of the upper right corner and the ceiling of the lower left corner. Because all lattice points have integer values, they can only lie in this shrunken square: 
```
Graphics[{Red, Rectangle[{-5.5, -5.5}, {5.5, 5.5}], 
  Lighter@Red, Rectangle[{-5.5, -5.5} // Ceiling, {5.5, 5.5} // Floor],
  Black, Point /@ Tuples[Range[-6, 6], 2]}]
```
![Shrunk square graphed on top of original square](/files/squarelatticeshrunk.svg)

Rectangles also work with the above formulation. It applies to the 3D case by taking a 3D range. Formulated for the n-dimensional case: {(p&#8321;,p&#8322;,...,p<sub>n</sub>) | p&#8321;&#8714; \[r&#185;&#8321;,r&#185;&#8322;\], p&#8322;&#8714; \[r&#178;&#8321;,r&#178;&#8322;\],...,p<sub>n</sub>&#8714;\[r<sup>n</sup>&#8321;,r<sup>n</sup>&#8322;\]} where p is a lattice point and r is the range from the lowest to highest corner (superscript of r is the dimension here, sorry for any confusion).

#### Time analysis

Finding all the lattice points in an n-dimensional hypercube is O(n) due to the requirement of taking the floor and ceiling of the corners. Counting the lattice points in a hypercube thus also takes O(n) time.

### Hyperspheres: much harder

Counting the lattice points inside a hypersphere is not as simple. Consider the 2D case, known as the [Gauss circle problem](https://en.wikipedia.org/wiki/Gauss_circle_problem):

`Graphics[{Red, Disk[{0, 0}, 10], Black, Point /@ Tuples[Range[-10, 10], 2]}]`
![Circle graphed with lattice points](/files/circlelattice.svg)

There are a lot of methods that can work. One simple way is to take the escribed square, find the contained lattice points, and filter out those that do not lie in the circle.

```
Graphics[{LightGray, Rectangle[{-10, -10}, {10, 10}], Red, 
   Disk[{0, 0}, 10], Black, Point /@ Tuples[Range[-10, 10], 2]}]
```
![Circle graphed with lattice points and an escribed square](/files/escribedsquare.svg)

Unfortunately, this does not perform too well. Each lattice point in the square has to be checked, taking O(r&#178;) time. It gets worse with higher dimensions, taking O(r<sup>n</sup>) time for the n-dimensional case.

Runtime can be improved by only evaluating on the lattice points between the inscribed square and the escribed square:
```
Graphics[{LightGray, Rectangle[{-10, -10}, {10, 10}], Red, 
   Disk[{0, 0}, 10], Darker@Red, 
   Rectangle[{-10, -10}/Sqrt[2], {10, 10}/Sqrt[2]], Black, 
   Point /@ Tuples[Range[-10, 10], 2]}]
```
![Circle graphed with lattice points an escribed square, and an inscribed square](/files/escribedsquareandinscribedsquare.svg)

But this is still taking O(r<sup>n</sup>) time. To improve this time bound, I tried and failed with many ideas, like a [midpoint circle algorithm](https://en.wikipedia.org/wiki/Midpoint_circle_algorithm) adaptation, flood filling, scanlines, and appoximating the circle as an inscribed regular polygon with n sides. In the end, I settled on a method that takes time proportional to the hypersurface and can be used to enumerate and count all points.

First, it finds the lattice points in the inscribed square. Then, to handle the remaining curved sections, it iterates over the sides of the shrunken inscribed square to build a new range of points to add. For each point on a side, it solves the circle equation treating the constant dimension (y if the side is horizontal, x if vertical) as missing to find the corresponding point on the circle. The range of lattice points from the side point to the circle point is then added.

I [implemented the method in Mathematica](/files/circle_lattice.pdf) and verified it up to a radius of 1000.


#### Time analysis

The method takes O(2<sup>n</sup>d<sup>(3-d)/2</sup>r<sup>d-1</sup>) which takes a linear O(r) time for the 2D case.


## Definitions
*Lattice point* a point whose coordinates are integers: p &#8714; &#8484;<sup>n</sup>.
*Hypersphere* an n-dimensional shape of points within a distance r from a point p: {x | &#8721;<sup>n</sup><sub>i=0</sub> (x<sub>i</sub>-p<sub>i</sub>)<sup>2</sup> &le; r<sup>2</sup>}.
*Hypercube* an n-dimensional cube. Square in 2D, Cube in 3D.
