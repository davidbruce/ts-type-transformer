interface GenericInterfaceTest<T> {
    name: T;
}
interface AesCtrParams extends GenericInterfaceTest<String> {
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
