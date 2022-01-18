set terminal png size 600,500 enhanced font 'Verdana,10'

# Axes label
#set xlabel 'Fehlerrate'
#set ylabel 'Effizienz'
# Axes ranges
#set xrange [0:1]
#set yrange [-1.5:1.5]

#set xtics 0.2
#set ytics 0.2


set output 'points.png'
plot "points.txt" using 1:2:3 with image

set output 'dots.png'
plot "dots.txt" using 1:2:3 with image
