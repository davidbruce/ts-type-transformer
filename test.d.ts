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
interface AesCtrParams extends Algorithm {
    counter: BufferSource;
    length: number;
}
interface Algorithm {
    name: string;
}
