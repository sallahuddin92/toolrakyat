import { SidebarLayout } from "./_components/SidebarLayout";

export default function AkaunKemasLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return <SidebarLayout>{children}</SidebarLayout>;
}
