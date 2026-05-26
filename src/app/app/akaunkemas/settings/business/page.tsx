import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";

const formFields = [
  { id: "businessName", label: "Business Name", placeholder: "My SME Sdn Bhd" },
  { id: "registrationNumber", label: "Registration Number", placeholder: "202401001234" },
  { id: "address", label: "Address", placeholder: "123, Jalan Example, Kuala Lumpur" },
  { id: "phone", label: "Phone", placeholder: "+60 12-345 6789" },
  { id: "email", label: "Email", placeholder: "hello@mysme.com" },
];

export default function BusinessSettingsPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight text-slate-900">Business Settings</h1>
        <p className="text-sm text-slate-500">
          Update your business profile and registration details.
        </p>
      </div>

      <Card className="max-w-2xl rounded-2xl">
        <CardHeader>
          <CardTitle className="text-base">Business Profile</CardTitle>
        </CardHeader>
        <CardContent>
          <form className="space-y-4">
            {formFields.map((field) => (
              <div key={field.id} className="space-y-1.5">
                <Label htmlFor={field.id}>{field.label}</Label>
                <Input
                  id={field.id}
                  disabled
                  placeholder={field.placeholder}
                />
              </div>
            ))}
            <Button disabled className="mt-6">
              Save
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
