export const fibonacci = (num) => {
    var a = 1, b = 0;

    while (num) {
        [a, b] = [a + b, a];
        num -= 1;
    }

    return b;
}

