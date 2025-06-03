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
interface AesCtrParams extends GenericInterfaceTest<String> {
    counter: BufferSource;
    length: number;
}
interface GenericInterfaceTest<T> {
    name: T;
}
