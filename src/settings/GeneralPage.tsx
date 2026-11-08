import { COUNTRIES, useCountry, WORLDWIDE } from "@/lib/country";
import { ChoiceRow } from "./controls";
import { Section } from "./rows";

export default function GeneralPage() {
  const [country, setCountry] = useCountry();

  return (
    <Section title="General">
      <ChoiceRow
        title="Country"
        summary="Charts and recommendations follow it"
        // nothing chosen reads as worldwide, which is what the charts use then
        value={country || WORLDWIDE}
        options={COUNTRIES}
        onChange={setCountry}
      />
    </Section>
  );
}
