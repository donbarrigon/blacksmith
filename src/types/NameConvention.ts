export interface NameConvention {
  // input: service_item
  singular: {
    snake: string // service_item
    camel: string // serviceItem
    pascal: string // ServiceItem
    kebab: string // service-item
    text: string // service item
    title: string // Service Item
  }

  plural: {
    snake: string // service_items
    camel: string // serviceItems
    pascal: string // ServiceItems
    kebab: string // service-items
    text: string // service items
    title: string // Service Items
  }

  original: string // service_item

  // input: DonBarrigon/spa_nails
  project: string // spa-nails
  gitUser: string // donbarrigon
  gomod: string // donbarrigon/spa_nails
}
