set terminal png size 600,500 enhanced font 'Verdana,10'

set output 'dense.png'
plot "points.txt" using 1:2:4 with image title "Dense"

set output 'sparse.png'
plot "points.txt" using 1:2:5 with image title "Sparse"

set output 'dots.png'
plot "points.txt" using 1:2:3 with image title "Input"
