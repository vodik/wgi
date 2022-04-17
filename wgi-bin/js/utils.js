export const fibonacci = (num) => {
    var a = 1, b = 0;

    for (; num; --num) {
        [a, b] = [a + b, a];
    }

    return b;
}

