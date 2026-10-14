export const ENVIRONMENTS = [
  {
    value: "local",
    label: "Local",
    token: "gray",
    dotClass: "bg-tag-gray",
    badgeClass: "border-tag-gray text-tag-gray",
  },
  {
    value: "development",
    label: "Development",
    token: "blue",
    dotClass: "bg-tag-blue",
    badgeClass: "border-tag-blue text-tag-blue",
  },
  {
    value: "testing",
    label: "Testing",
    token: "teal",
    dotClass: "bg-tag-teal",
    badgeClass: "border-tag-teal text-tag-teal",
  },
  {
    value: "staging",
    label: "Staging",
    token: "orange",
    dotClass: "bg-tag-orange",
    badgeClass: "border-tag-orange text-tag-orange",
  },
  {
    value: "production",
    label: "Production",
    token: "red",
    dotClass: "bg-tag-red",
    badgeClass: "border-tag-red text-tag-red",
  },
];

export function environmentFor(value) {
  return (
    ENVIRONMENTS.find((environment) => environment.value === value) ?? {
      value,
      label: "Unknown",
      token: "gray",
      dotClass: "bg-tag-gray",
      badgeClass: "border-tag-gray text-tag-gray",
    }
  );
}
