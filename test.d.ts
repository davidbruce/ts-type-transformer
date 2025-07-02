interface GenericInterfaceTest<T, K> {
    name: T;
    place: K;
}
interface AesCtrParams extends GenericInterfaceTest<String, Number> {
    counter: BufferSource;
    length: number;
}
interface AddressErrors {
    addressLine?: string;
    city?: string;
    country?: string;
    dependentLocality?: string;
    organization?: string;
    phone?: string;
    postalCode?: string;
    recipient?: string;
    region?: string;
    sortingCode?: string;
}
