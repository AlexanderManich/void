typedef struct BitScanResult BitScanResult;
struct BitScanResult {
    b32 found;
    u32 index;
};

BitScanResult find_least_significant_set_bit(u32 value) {
    BitScanResult result = {};

    if (value != 0) {
        result.found = 1;
        result.index = __builtin_ctzl(value);
    }

    return result;
}

